use std::{
    pin::Pin,
    sync::{Arc, Mutex},
};

use rdif_block::{DriverGeneric, Event, IReadQueue, Interface, RequestId};

pub struct RamBlk {
    block_size: usize,
    num_blocks: usize,
    inner: Arc<Mutex<Inner>>,
}

struct Handle {
    inner: Arc<Mutex<Inner>>,
}

impl Handle {
    fn handle(&self) -> Option<Event> {
        let event = {
            let mut g = self.inner.lock().unwrap();
            g.event.take()
        };
        event.map(|e| Event {
            queue_id: 1,
            queue_elem_id: e.req_id,
        })
    }
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
                            *i = 1;
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
            inner: self.inner.clone(),
        }))
    }

    pub fn handle(&mut self) -> Handle {
        Handle {
            inner: self.inner.clone(),
        }
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
    inner: Arc<Mutex<Inner>>,
}

impl RamReadQueue {
    fn id(&self) -> usize {
        1
    }

    fn block_size(&self) -> usize {
        self.block_size
    }

    fn request_block(&mut self, block_id: usize) -> Result<RequestId, rdif_base::io::Error> {
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

    fn check_request(&mut self, req: &RequestId) -> Result<Vec<u8>, rdif_base::io::Error> {
        let ready = {
            let mut g = self.inner.lock().unwrap();
            g.ready.take()
        };
        if let Some(id) = ready {
            if id.req_id != req.request_id {
                return Err(rdif_base::io::Error {
                    kind: rdif_base::io::ErrorKind::InvalidParameter { name: "req" },
                    success_pos: 0,
                });
            }
            let g = self.inner.lock().unwrap();
            let start = id.blk_id * self.block_size;
            let end = start + self.block_size;
            return Ok(g.data[start..end].to_vec());
        }
        Err(rdif_base::io::Error {
            kind: rdif_base::io::ErrorKind::Interrupted,
            success_pos: 0,
        })
    }

    fn read_block(
        &mut self,
        block_id: usize,
    ) -> Result<
        impl Future<Output = Result<Vec<u8>, rdif_base::io::Error>> + '_,
        rdif_base::io::Error,
    > {
        let req = self.request_block(block_id)?;
        Ok(ReadFuture { queue: self, req })
    }

    fn add_wake(&mut self, req: &RequestId, waker: &std::task::Waker) {}
}

pub struct ReadFuture<'a> {
    req: RequestId,
    queue: &'a mut RamReadQueue,
}

impl<'a> Future for ReadFuture<'a> {
    type Output = Result<Vec<u8>, rdif_base::io::Error>;

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.queue.check_request(&self.req) {
            Ok(data) => std::task::Poll::Ready(Ok(data)),
            Err(e) => {
                if matches!(e.kind, rdif_base::io::ErrorKind::Interrupted) {
                    cx.waker().wake_by_ref();
                    std::task::Poll::Pending
                } else {
                    std::task::Poll::Ready(Err(e))
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let mut ram = RamBlk::new(16, 1024);
    let mut rq = ram.read_queue().unwrap();

    let handle = ram.handle();

    std::thread::spawn(move || {
        loop {
            handle.handle();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    });

    let req = rq.request_block(1).unwrap();

    loop {
        if let Ok(data) = rq.check_request(&req) {
            println!("data :{data:?}");
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    println!("done");
}
