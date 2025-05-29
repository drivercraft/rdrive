#![no_std]

extern crate alloc;

use alloc::boxed::Box;

pub use rdif_base::io;
pub use rdif_base::io::async_trait;
pub use rdif_base::{DriverGeneric, KError};

pub trait Interface: DriverGeneric {
    /// Call in irq handler.
    fn handle_irq(&mut self);
    /// [`Sender`] will be given back when dropped.
    fn take_tx(&mut self) -> Option<Box<dyn io::Write>>;
    /// [`Reciever`] will be given back when dropped.
    fn take_rx(&mut self) -> Option<Box<dyn io::Read>>;
}

/// Serial error kind.
///
/// This represents a common set of serial operation errors. HAL implementations are
/// free to define more specific or additional error types. However, by providing
/// a mapping to these common serial errors, generic code can still react to them.
#[derive(thiserror::Error, Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum SerialError {
    /// The peripheral receive buffer was overrun.
    #[error("The peripheral receive buffer was overrun.")]
    Overrun,
    /// Received data does not conform to the peripheral configuration.
    /// Can be caused by a misconfigured device on either end of the serial line.
    #[error("Received data does not conform to the peripheral configuration.")]
    FrameFormat,
    /// Parity check failed.
    #[error("Parity check failed.")]
    Parity,
    /// Serial line is too noisy to read valid data.
    #[error("Serial line is too noisy to read valid data.")]
    Noise,
    /// Device was closed.
    #[error("Device was closed.")]
    Closed,
    /// A different error occurred. The original error may contain more information.
    #[error("Unknown error.")]
    Other,
}

impl From<SerialError> for io::ErrorKind {
    fn from(value: SerialError) -> Self {
        match value {
            SerialError::Closed => io::ErrorKind::BrokenPipe,
            _ => io::ErrorKind::Other(Box::new(value)),
        }
    }
}
