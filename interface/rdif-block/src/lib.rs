#![no_std]

pub use rdif_base::{DriverGeneric, KError, io};

/// Operations that require a block storage device driver to implement.
pub trait Interface: DriverGeneric {
    /// The number of blocks in this storage device.
    ///
    /// The total size of the device is `num_blocks() * block_size()`.
    fn num_blocks(&self) -> usize;
    /// The size of each block in bytes.
    fn block_size(&self) -> usize;

    /// Reads blocked data from the given block.
    ///
    /// The size of the buffer may exceed the block size, in which case multiple
    /// contiguous blocks will be read.
    fn read_block(&mut self, block_id: usize, buf: &mut [u8]) -> Result<(), io::Error>;

    /// Writes blocked data to the given block.
    ///
    /// The size of the buffer may exceed the block size, in which case multiple
    /// contiguous blocks will be written.
    fn write_block(&mut self, block_id: usize, buf: &[u8]) -> Result<(), io::Error>;

    /// Flushes the device to write all pending data to the storage.
    fn flush(&mut self) -> Result<(), io::Error>;
}
