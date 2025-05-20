#![no_std]

extern crate alloc;

pub use rdif_base::{DriverGeneric, DriverResult};
pub use alloc::boxed::Box;

pub type Hardware = Box<dyn BlockDriver>;

/// Operations that require a block storage device driver to implement.
pub trait BlockDriver: DriverGeneric  {
    /// The number of blocks in this storage device.
    ///
    /// The total size of the device is `num_blocks() * block_size()`.
    fn num_blocks(&self) -> u64;
    /// The size of each block in bytes.
    fn block_size(&self) -> usize;

    /// Reads blocked data from the given block.
    ///
    /// The size of the buffer may exceed the block size, in which case multiple
    /// contiguous blocks will be read.
    fn read_block(&mut self, block_id: u64, buf: &mut [u8]) -> DriverResult;

    /// Writes blocked data to the given block.
    ///
    /// The size of the buffer may exceed the block size, in which case multiple
    /// contiguous blocks will be written.
    fn write_block(&mut self, block_id: u64, buf: &[u8]) -> DriverResult;

    /// Flushes the device to write all pending data to the storage.
    fn flush(&mut self) -> DriverResult;
}

