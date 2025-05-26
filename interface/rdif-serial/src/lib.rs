#![no_std]

extern crate alloc;

use alloc::boxed::Box;

pub use futures::future::LocalBoxFuture;
pub use rdif_base::{DriverGeneric, ErrorBase};

pub trait Sender: Send {
    fn write(&mut self, buf: &[u8]) -> Result<usize, SerialError>;
    fn write_all<'a>(&'a mut self, buf: &'a [u8]) -> LocalBoxFuture<'a, Result<(), SerialError>>;
}

pub trait Reciever: Send {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, SerialError>;
    fn read_all<'a>(&'a mut self, buf: &'a mut [u8])
    -> LocalBoxFuture<'a, Result<(), SerialError>>;
}

pub trait Interface: DriverGeneric {
    /// Call in irq handler.
    fn handle_irq(&mut self);
    /// [`Sender`] will be given back when dropped.
    fn take_tx(&mut self) -> Option<Box<dyn Sender>>;
    /// [`Reciever`] will be given back when dropped.
    fn take_rx(&mut self) -> Option<Box<dyn Reciever>>;
}

/// Serial error kind.
///
/// This represents a common set of serial operation errors. HAL implementations are
/// free to define more specific or additional error types. However, by providing
/// a mapping to these common serial errors, generic code can still react to them.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[non_exhaustive]
pub enum SerialError {
    /// The peripheral receive buffer was overrun.
    Overrun,
    /// Received data does not conform to the peripheral configuration.
    /// Can be caused by a misconfigured device on either end of the serial line.
    FrameFormat,
    /// Parity check failed.
    Parity,
    /// Serial line is too noisy to read valid data.
    Noise,
    /// Device was closed.
    Closed,
    /// A different error occurred. The original error may contain more information.
    Other,
}

impl core::error::Error for SerialError {}

impl core::fmt::Display for SerialError {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Overrun => write!(f, "The peripheral receive buffer was overrun"),
            Self::Parity => write!(f, "Parity check failed"),
            Self::Noise => write!(f, "Serial line is too noisy to read valid data"),
            Self::FrameFormat => write!(
                f,
                "Received data does not conform to the peripheral configuration"
            ),
            Self::Closed => write!(f, "Device was closed"),
            Self::Other => write!(
                f,
                "A different error occurred. The original error may contain more information"
            ),
        }
    }
}
