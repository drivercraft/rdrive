use core::{
    cell::UnsafeCell,
    fmt::Debug,
    ops::{Deref, DerefMut},
    task::Poll,
};

use alloc::{
    boxed::Box,
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    sync::Arc,
    vec::Vec,
};
use dma_api::{DBuff, DVecConfig, DVecPool, Direction};
use futures::task::AtomicWaker;

use crate::{BlkError, Buffer, IReadQueue, IWriteQueue, Interface, RequestId};

pub struct Block {
    inner: Arc<BlockInner>,
}

struct QueueWeakerMap(UnsafeCell<BTreeMap<usize, Arc<AtomicWaker>>>);

impl QueueWeakerMap {
    fn new() -> Self {
        Self(UnsafeCell::new(BTreeMap::new()))
    }

    fn register(&self, queue_id: usize) -> Arc<AtomicWaker> {
        let waker = Arc::new(AtomicWaker::new());
        unsafe { &mut *self.0.get() }.insert(queue_id, waker.clone());
        waker
    }

    fn wake(&self, queue_id: usize) {
        if let Some(waker) = unsafe { &*self.0.get() }.get(&queue_id) {
            waker.wake();
        }
    }
}

struct BlockInner {
    interface: UnsafeCell<Box<dyn Interface>>,
    rx_waker_map: QueueWeakerMap,
    tx_waker_map: QueueWeakerMap,
}

unsafe impl Send for BlockInner {}
unsafe impl Sync for BlockInner {}

struct IrqGuard<'a> {
    enabled: bool,
    inner: &'a Block,
}

impl<'a> Drop for IrqGuard<'a> {
    fn drop(&mut self) {
        if self.enabled {
            self.inner.interface().irq_enable();
        }
    }
}

impl Block {
    pub fn new(iterface: impl Interface) -> Self {
        Self {
            inner: Arc::new(BlockInner {
                interface: UnsafeCell::new(Box::new(iterface)),
                rx_waker_map: QueueWeakerMap::new(),
                tx_waker_map: QueueWeakerMap::new(),
            }),
        }
    }

    pub fn open(&mut self) -> Result<(), rdif_base::KError> {
        self.interface().open()
    }

    pub fn close(&mut self) -> Result<(), rdif_base::KError> {
        self.interface().close()
    }

    #[allow(clippy::mut_from_ref)]
    fn interface(&self) -> &mut Box<dyn Interface> {
        unsafe { &mut *self.inner.interface.get() }
    }

    fn irq_guard(&self) -> IrqGuard<'_> {
        let enabled = self.interface().irq_is_enabled();
        if enabled {
            self.interface().irq_disable();
        }
        IrqGuard {
            enabled,
            inner: self,
        }
    }

    pub fn new_read_queue_with_pool_cap(&mut self, cap: usize) -> Option<ReadQueue> {
        let irq_guard = self.irq_guard();
        let queue = self.interface().new_read_queue()?;
        let queue_id = queue.id();
        let config = queue.buff_config();
        let waker = self.inner.rx_waker_map.register(queue_id);
        drop(irq_guard);

        Some(ReadQueue::new(
            queue,
            waker,
            DVecConfig {
                dma_mask: config.dma_mask,
                align: config.align,
                size: config.size,
                direction: Direction::FromDevice,
            },
            cap,
        ))
    }

    pub fn new_read_queue(&mut self) -> Option<ReadQueue> {
        self.new_read_queue_with_pool_cap(32)
    }

    pub fn irq_handler(&self) -> IrqHandler {
        IrqHandler {
            inner: self.inner.clone(),
        }
    }

    pub fn new_write_queue(&mut self) -> Option<WriteQueue> {
        let irq_guard = self.irq_guard();
        let queue = self.interface().new_write_queue()?;
        let queue_id = queue.id();
        let waker = self.inner.tx_waker_map.register(queue_id);
        drop(irq_guard);

        Some(WriteQueue::new(queue, waker))
    }
}

pub struct IrqHandler {
    inner: Arc<BlockInner>,
}

unsafe impl Sync for IrqHandler {}

impl IrqHandler {
    pub fn handle(&self) {
        let iface = unsafe { &mut *self.inner.interface.get() };
        let event = iface.handle_irq();
        for id in event.rx_queue.iter() {
            self.inner.rx_waker_map.wake(id);
        }
        for id in event.tx_queue.iter() {
            self.inner.tx_waker_map.wake(id);
        }
    }
}

pub struct ReadQueue {
    interface: Box<dyn IReadQueue>,
    waker: Arc<AtomicWaker>,
    pool: DVecPool,
}

pub struct WriteQueue {
    interface: Box<dyn IWriteQueue>,
    waker: Arc<AtomicWaker>,
}

pub struct BlockData {
    block_id: usize,
    data: DBuff,
}

impl ReadQueue {
    fn new(
        iterface: Box<dyn IReadQueue>,
        waker: Arc<AtomicWaker>,
        config: DVecConfig,
        cap: usize,
    ) -> Self {
        Self {
            interface: iterface,
            waker,
            pool: DVecPool::new_pool(config, cap),
        }
    }

    pub fn id(&self) -> usize {
        self.interface.id()
    }

    pub fn num_blocks(&self) -> usize {
        self.interface.num_blocks()
    }

    pub fn block_size(&self) -> usize {
        self.interface.block_size()
    }

    pub async fn read_blocks(
        &mut self,
        block_id_ls: impl AsRef<[usize]>,
    ) -> Vec<Result<BlockData, BlkError>> {
        let block_id_ls = block_id_ls.as_ref().to_vec();
        ReadFuture::new(self, block_id_ls).await
    }
}
pub struct ReadFuture<'a> {
    queue: &'a mut ReadQueue,
    blk_ls: Vec<usize>,
    requested: BTreeMap<usize, Option<DBuff>>,
    map: BTreeMap<usize, RequestId>,
    results: BTreeMap<usize, Result<BlockData, BlkError>>,
}

impl<'a> ReadFuture<'a> {
    fn new(queue: &'a mut ReadQueue, blk_ls: Vec<usize>) -> Self {
        Self {
            queue,
            blk_ls,
            requested: BTreeMap::new(),
            map: BTreeMap::new(),
            results: BTreeMap::new(),
        }
    }
}

impl<'a> core::future::Future for ReadFuture<'a> {
    type Output = Vec<Result<BlockData, BlkError>>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let this = self.get_mut();

        for &blk_id in &this.blk_ls {
            if this.results.contains_key(&blk_id) {
                continue;
            }

            if this.requested.contains_key(&blk_id) {
                continue;
            }

            match this.queue.pool.alloc() {
                Ok(buff) => {
                    match this.queue.interface.request_block(
                        blk_id,
                        Buffer {
                            virt: buff.as_ptr(),
                            bus: buff.bus_addr(),
                            size: buff.len(),
                        },
                    ) {
                        Ok(req_id) => {
                            this.map.insert(blk_id, req_id);
                            this.requested.insert(blk_id, Some(buff));
                        }
                        Err(BlkError::WouldBlock) => {
                            this.queue.waker.register(cx.waker());
                            return Poll::Pending;
                        }
                        Err(e) => {
                            this.results.insert(blk_id, Err(e));
                        }
                    }
                }
                Err(e) => {
                    this.results.insert(blk_id, Err(e.into()));
                }
            }
        }

        for (blk_id, buff) in &mut this.requested {
            if this.results.contains_key(blk_id) {
                continue;
            }

            let req_id = this.map[blk_id];

            match this.queue.interface.check_request(req_id) {
                Ok(_) => {
                    this.results.insert(
                        *blk_id,
                        Ok(BlockData {
                            block_id: *blk_id,
                            data: buff.take().unwrap(),
                        }),
                    );
                }
                Err(BlkError::WouldBlock) => {
                    this.queue.waker.register(cx.waker());
                    return Poll::Pending;
                }
                Err(e) => {
                    this.results.insert(*blk_id, Err(e));
                }
            }
        }

        let mut out = Vec::with_capacity(this.blk_ls.len());
        for blk_id in &this.blk_ls {
            let result = this.results.remove(blk_id).unwrap();
            out.push(result);
        }
        Poll::Ready(out)
    }
}

impl WriteQueue {
    fn new(interface: Box<dyn IWriteQueue>, waker: Arc<AtomicWaker>) -> Self {
        Self { interface, waker }
    }

    pub fn id(&self) -> usize {
        self.interface.id()
    }

    pub fn num_blocks(&self) -> usize {
        self.interface.num_blocks()
    }

    pub fn block_size(&self) -> usize {
        self.interface.block_size()
    }

    /// Write multiple blocks. Caller provides owned Vec<u8> buffers for each block.
    pub async fn write_blocks<T, R>(&mut self, block_vecs: T) -> Vec<Result<(), BlkError>>
    where
        T: AsRef<[(usize, R)]>,
        R: AsRef<[u8]>,
    {
        let block_vecs: Vec<(usize, &[u8])> = block_vecs
            .as_ref()
            .iter()
            .map(|(id, buf)| (*id, buf.as_ref()))
            .collect();
        WriteFuture::new(self, block_vecs).await
    }
}

pub struct WriteFuture<'a, 'b> {
    queue: &'a mut WriteQueue,
    req_ls: Vec<(usize, &'b [u8])>,
    requested: BTreeSet<usize>,
    map: BTreeMap<usize, RequestId>,
    results: BTreeMap<usize, Result<(), BlkError>>,
}

impl<'a, 'b> WriteFuture<'a, 'b> {
    fn new(queue: &'a mut WriteQueue, req_ls: Vec<(usize, &'b [u8])>) -> Self {
        Self {
            queue,
            req_ls,
            requested: BTreeSet::new(),
            map: BTreeMap::new(),
            results: BTreeMap::new(),
        }
    }
}

impl<'a, 'b> core::future::Future for WriteFuture<'a, 'b> {
    type Output = Vec<Result<(), BlkError>>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let this = self.get_mut();
        for &(blk_id, buff) in &this.req_ls {
            if this.results.contains_key(&blk_id) {
                continue;
            }

            if this.requested.contains(&blk_id) {
                continue;
            }

            match this.queue.interface.request_block(blk_id, buff) {
                Ok(req_id) => {
                    this.map.insert(blk_id, req_id);
                    this.requested.insert(blk_id);
                }
                Err(BlkError::WouldBlock) => {
                    this.queue.waker.register(cx.waker());
                    return Poll::Pending;
                }
                Err(e) => {
                    this.results.insert(blk_id, Err(e));
                }
            }
        }

        for blk_id in this.requested.iter() {
            if this.results.contains_key(blk_id) {
                continue;
            }

            let req_id = this.map[blk_id];

            match this.queue.interface.check_request(req_id) {
                Ok(_) => {
                    this.results.insert(*blk_id, Ok(()));
                }
                Err(BlkError::WouldBlock) => {
                    this.queue.waker.register(cx.waker());
                    return Poll::Pending;
                }
                Err(e) => {
                    this.results.insert(*blk_id, Err(e));
                }
            }
        }

        let mut out = Vec::with_capacity(this.req_ls.len());
        for (blk_id, _) in &this.req_ls {
            let result = this.results.remove(blk_id).unwrap();
            out.push(result);
        }
        Poll::Ready(out)
    }
}

impl Debug for BlockData {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("BlockData")
            .field("block_id", &self.block_id)
            .field("data", &self.data.as_ref())
            .finish()
    }
}

impl BlockData {
    pub fn block_id(&self) -> usize {
        self.block_id
    }
}

impl Deref for BlockData {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.data.as_ref()
    }
}

impl DerefMut for BlockData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { core::slice::from_raw_parts_mut(self.data.as_ptr(), self.data.len()) }
    }
}
