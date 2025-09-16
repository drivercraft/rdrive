use std::{
    pin::Pin,
    sync::{Arc, Mutex},
};

use rdif_block::{Block, DriverGeneric, Event as REvent, IReadQueue, IdList, Interface, RequestId};

pub struct RamBlk {
    block_size: usize,
    num_blocks: usize,
    inner: Arc<Mutex<Inner>>,
}

#[derive(Clone, Copy)]
struct Elem {
    req_id: usize,
    blk_id: usize,
}

struct Inner {
    block_size: usize,
    perper: Option<Elem>,
    ready: Option<Elem>,
    data: Vec<u8>,
    event: Option<Elem>,
}

impl RamBlk {
    pub fn new(block_size: usize, num_blocks: usize) -> Self {
        let inner = Arc::new(Mutex::new(Inner {
            block_size,
            perper: None,
            data: vec![0; block_size * num_blocks],
            ready: None,
            event: None,
        }));

        std::thread::spawn({
            let inner = inner.clone();
            move || loop {
                let perper_id = {
                    let mut g = inner.lock().unwrap();
                    g.perper.take()
                };
                std::thread::sleep(std::time::Duration::from_millis(10));
                if let Some(id) = perper_id {
                    let start = id.blk_id * block_size;
                    let end = start + block_size;
                    {
                        let mut g = inner.lock().unwrap();
                        for i in g.data[start..end].iter_mut() {
                            *i = id.blk_id as u8;
                        }
                        g.ready = Some(id);
                        g.event = Some(id)
                    }
                }
            }
        });

        Self {
            block_size,
            num_blocks,
            inner,
        }
    }

    pub fn read_queue(&mut self) -> Option<Box<RamReadQueue>> {
        Some(Box::new(RamReadQueue {
            block_size: self.block_size,
            num_blocks: self.num_blocks,
            inner: self.inner.clone(),
        }))
    }
}

impl DriverGeneric for RamBlk {
    fn open(&mut self) -> Result<(), rdif_base::KError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), rdif_base::KError> {
        Ok(())
    }
}

pub struct RamReadQueue {
    block_size: usize,
    num_blocks: usize,
    inner: Arc<Mutex<Inner>>,
}

impl RamReadQueue {
    fn id(&self) -> usize {
        0
    }

    fn block_size(&self) -> usize {
        self.block_size
    }
    fn request_block(&mut self, block_id: usize) -> Result<RequestId, rdif_base::io::Error> {
        if block_id >= self.num_blocks {
            return Err(rdif_base::io::Error {
                kind: rdif_base::io::ErrorKind::InvalidParameter { name: "block_id" },
                success_pos: 0,
            });
        }
        let req_id = 1;
        {
            let mut g = self.inner.lock().unwrap();
            if g.perper.is_some() {
                return Err(rdif_base::io::Error {
                    kind: rdif_base::io::ErrorKind::Interrupted,
                    success_pos: 0,
                });
            }
            g.perper = Some(Elem {
                req_id,
                blk_id: block_id,
            });
        }
        Ok(RequestId::new(req_id))
    }

    fn check_request(
        &mut self,
        req: RequestId,
    ) -> Result<Box<dyn rdif_block::Buffer>, rdif_base::io::Error> {
        let ready = {
            let mut g = self.inner.lock().unwrap();
            g.ready.take()
        };
        if let Some(id) = ready {
            let rid: usize = usize::from(req);
            if id.req_id != rid {
                return Err(rdif_base::io::Error {
                    kind: rdif_base::io::ErrorKind::InvalidParameter { name: "req" },
                    success_pos: 0,
                });
            }
            let g = self.inner.lock().unwrap();
            let start = id.blk_id * self.block_size;
            let end = start + self.block_size;
            let buf = g.data[start..end].to_vec();
            return Ok(Box::new(BufferVec(buf)));
        }
        Err(rdif_base::io::Error {
            kind: rdif_base::io::ErrorKind::Interrupted,
            success_pos: 0,
        })
    }
}

impl rdif_block::IReadQueue for RamReadQueue {
    fn id(&self) -> usize {
        self.id()
    }
    fn num_blocks(&self) -> usize {
        self.num_blocks
    }
    fn block_size(&self) -> usize {
        self.block_size()
    }
    fn request_block(&mut self, block_id: usize) -> Result<RequestId, rdif_base::io::Error> {
        self.request_block(block_id)
    }
    fn check_request(
        &mut self,
        request: RequestId,
    ) -> Result<Box<dyn rdif_block::Buffer>, rdif_base::io::Error> {
        self.check_request(request)
    }
}

// A small Buffer implementation to return heap-backed data.
pub struct BufferVec(Vec<u8>);

impl AsRef<[u8]> for BufferVec {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsMut<[u8]> for BufferVec {
    fn as_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

impl rdif_block::Buffer for BufferVec {}

impl Interface for RamBlk {
    fn new_read_queue(&mut self) -> Option<Box<dyn IReadQueue>> {
        self.read_queue().map(|b| b as Box<dyn IReadQueue>)
    }

    fn irq_enable(&mut self) {}
    fn irq_disable(&mut self) {}
    fn irq_is_enabled(&self) -> bool {
        true
    }

    fn handle_irq(&mut self) -> REvent {
        let ev = {
            let mut g = self.inner.lock().unwrap();
            g.event.take()
        };
        if let Some(_e) = ev {
            let mut il = IdList::none();
            il.insert(0);
            REvent {
                rx_queue: il,
                tx_queue: IdList::none(),
            }
        } else {
            REvent::none()
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    // create a ram device with 16 byte blocks and 1024 blocks
    let mut ram = Block::new(RamBlk::new(16, 1024));

    // open device (no-op here)
    let _ = ram.open();

    // get a read queue via the new Interface API
    let mut rq = ram.new_read_queue().expect("read queue");

    // spawn a thread that polls the device handle and prints events
    let handle = ram.irq_handler();
    std::thread::spawn(move || {
        loop {
            handle.handle();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    });

    // request a block and asynchronously poll for completion
    let data = rq.read_blocks(&[1, 2]).await.unwrap();

    for b in data {
        println!("block: {:?}", b);
    }

    println!("done");
}
