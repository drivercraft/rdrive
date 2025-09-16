use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
};

use alloc::{boxed::Box, collections::btree_map::BTreeMap, sync::Arc, vec::Vec};
use futures::task::AtomicWaker;
use rdif_base::io;

use crate::{Buffer, IReadQueue, Interface, RequestId};

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
            }),
        }
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

    pub fn new_read_queue(&mut self) -> Option<ReadQueue> {
        let irq_guard = self.irq_guard();
        let queue = self.interface().new_read_queue()?;
        let queue_id = queue.id();
        let waker = self.inner.rx_waker_map.register(queue_id);
        drop(irq_guard);

        Some(ReadQueue::new(queue, waker))
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
    }
}

pub struct ReadQueue {
    iterface: Box<dyn IReadQueue>,
    waker: Arc<AtomicWaker>,
}

impl ReadQueue {
    fn new(iterface: Box<dyn IReadQueue>, waker: Arc<AtomicWaker>) -> Self {
        Self { iterface, waker }
    }

    pub fn id(&self) -> usize {
        self.iterface.id()
    }

    pub fn num_blocks(&self) -> usize {
        self.iterface.num_blocks()
    }

    pub fn block_size(&self) -> usize {
        self.iterface.block_size()
    }

    pub fn request_block(&mut self, block_id: usize) -> Result<RequestId, io::Error> {
        self.iterface.request_block(block_id)
    }

    pub fn check_request(&mut self, req: RequestId) -> Result<Box<dyn Buffer>, io::Error> {
        self.iterface.check_request(req)
    }

    pub fn read_blocks(
        &mut self,
        block_id_ls: impl AsRef<[usize]>,
    ) -> impl Future<Output = Result<Vec<BlockData>, io::Error>> + '_ {
        let block_id_ls = block_id_ls.as_ref().to_vec();
        let mut req_ls = Vec::with_capacity(block_id_ls.len());

        let mut err = None;

        for &id in &block_id_ls {
            match self.request_block(id) {
                Ok(r) => req_ls.push(r),
                Err(e) => {
                    err = Some(e);
                    break;
                }
            }
        }

        async move {
            if let Some(e) = err {
                return Err(e);
            }

            ReadFuture {
                queue: self,
                blk_id_ls: block_id_ls,
                req_id_ls: req_ls,
                completed: BTreeMap::new(),
            }
            .await
        }
    }
}

pub struct ReadFuture<'a> {
    queue: &'a mut ReadQueue,
    blk_id_ls: Vec<usize>,
    req_id_ls: Vec<RequestId>,
    completed: BTreeMap<usize, Box<dyn Buffer>>,
}

impl<'a> core::future::Future for ReadFuture<'a> {
    type Output = Result<Vec<BlockData>, io::Error>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let this = self.get_mut();

        for (blk_id, req_id) in this.blk_id_ls.iter().zip(this.req_id_ls.iter()) {
            if !this.completed.contains_key(blk_id) {
                match this.queue.check_request(*req_id) {
                    Ok(buf) => {
                        this.completed.insert(*blk_id, buf);
                    }
                    Err(e) => {
                        if matches!(e.kind, io::ErrorKind::Interrupted) {
                            this.queue.waker.register(cx.waker());
                            return core::task::Poll::Pending;
                        } else {
                            return core::task::Poll::Ready(Err(e));
                        }
                    }
                }
            }
        }

        core::task::Poll::Ready(Ok(this
            .blk_id_ls
            .iter()
            .map(|id| BlockData {
                block_id: *id,
                data: this.completed.remove(id).unwrap(),
            })
            .collect()))
    }
}

pub struct BlockData {
    block_id: usize,
    data: Box<dyn Buffer>,
}

impl BlockData {
    pub fn block_id(&self) -> usize {
        self.block_id
    }
}

impl Deref for BlockData {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.data.as_ref().as_ref()
    }
}

impl DerefMut for BlockData {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data.as_mut().as_mut()
    }
}
