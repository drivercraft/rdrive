#![no_std]

extern crate alloc;

use alloc::boxed::Box;
pub use rdif_base::{DriverGeneric, KError, io};

mod blk;

pub use blk::*;

pub use dma_api;

pub struct BuffConfig {
    pub dma_mask: u64,
    pub align: usize,
    pub size: usize,
}

#[derive(thiserror::Error, Debug)]
pub enum BlkError {
    #[error("Not supported")]
    NotSupported,
    #[error("Need retry")]
    Retry,
    #[error("No memory")]
    NoMemory,
    #[error("Out of bounds, block: {0}")]
    OutOfBounds(usize),
    #[error("Unknown: {0}")]
    Unknown(Box<dyn core::error::Error>),
}

impl From<BlkError> for io::ErrorKind {
    fn from(value: BlkError) -> Self {
        match value {
            BlkError::NotSupported => io::ErrorKind::Unsupported,
            BlkError::Retry => io::ErrorKind::Interrupted,
            BlkError::NoMemory => io::ErrorKind::OutOfMemory,
            BlkError::OutOfBounds(_) => io::ErrorKind::NotAvailable,
            BlkError::Unknown(e) => io::ErrorKind::Other(e),
        }
    }
}

impl From<dma_api::DError> for BlkError {
    fn from(value: dma_api::DError) -> Self {
        match value {
            dma_api::DError::NoMemory => BlkError::NoMemory,
            e => BlkError::Unknown(Box::new(e)),
        }
    }
}

/// Operations that require a block storage device driver to implement.
pub trait Interface: DriverGeneric {
    fn new_read_queue(&mut self) -> Option<Box<dyn IReadQueue>>;
    fn new_write_queue(&mut self) -> Option<Box<dyn IWriteQueue>>;

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

#[derive(Clone, Copy)]
pub struct Buffer {
    pub virt: *mut u8,
    pub bus: u64,
    pub size: usize,
}

impl Buffer {
    pub fn copy_from_slice(&mut self, src: &[u8]) {
        assert!(src.len() <= self.size);
        unsafe {
            core::ptr::copy_nonoverlapping(src.as_ptr(), self.virt, src.len());
        }
    }
}

pub trait IReadQueue: Send + 'static {
    fn id(&self) -> usize;
    fn num_blocks(&self) -> usize;
    fn block_size(&self) -> usize;
    fn buff_config(&self) -> BuffConfig;
    fn request_block(&mut self, block_id: usize, buff: Buffer) -> Result<RequestId, BlkError>;
    fn check_request(&mut self, request: RequestId) -> Result<(), BlkError>;
}

/// Write queue trait for block devices.
pub trait IWriteQueue: Send + 'static {
    fn id(&self) -> usize;
    fn num_blocks(&self) -> usize;
    fn block_size(&self) -> usize;

    fn request_block(&mut self, block_id: usize, buff: &[u8]) -> Result<RequestId, BlkError>;
    /// Check whether a previously requested write is complete.
    fn check_request(&mut self, request: RequestId) -> Result<(), BlkError>;
}
