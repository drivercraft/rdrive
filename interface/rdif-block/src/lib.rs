#![no_std]

extern crate alloc;

use alloc::boxed::Box;
pub use rdif_base::{DriverGeneric, KError, io};

mod blk;

pub use blk::*;

/// Operations that require a block storage device driver to implement.
pub trait Interface: DriverGeneric {
    fn new_read_queue(&mut self) -> Option<Box<dyn IReadQueue>>;

    fn irq_enable(&mut self);
    fn irq_disable(&mut self);
    fn irq_is_enabled(&self) -> bool;

    /// Handles an IRQ from the device, returning an event if applicable.
    fn handle_irq(&mut self) -> Event;
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct IdList(u64);

impl IdList {
    pub const fn none() -> Self {
        Self(0)
    }

    pub fn contains(&self, id: usize) -> bool {
        (self.0 & (1 << id)) != 0
    }

    pub fn insert(&mut self, id: usize) {
        self.0 |= 1 << id;
    }

    pub fn remove(&mut self, id: usize) {
        self.0 &= !(1 << id);
    }

    pub fn iter(&self) -> impl Iterator<Item = usize> {
        (0..64).filter(move |i| self.contains(*i))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Event {
    pub rx_queue: IdList,
    pub tx_queue: IdList,
}

impl Event {
    pub const fn none() -> Self {
        Self {
            rx_queue: IdList::none(),
            tx_queue: IdList::none(),
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RequestId(usize);

impl RequestId {
    pub fn new(id: usize) -> Self {
        Self(id)
    }
}

impl From<RequestId> for usize {
    fn from(value: RequestId) -> Self {
        value.0
    }
}

pub trait Buffer: AsMut<[u8]> + AsRef<[u8]> + Send + 'static {}

pub trait IReadQueue: Send + 'static {
    fn id(&self) -> usize;
    fn num_blocks(&self) -> usize;
    fn block_size(&self) -> usize;
    fn request_block(&mut self, block_id: usize) -> Result<RequestId, io::Error>;
    fn check_request(&mut self, request: RequestId) -> Result<Box<dyn Buffer>, io::Error>;
}
