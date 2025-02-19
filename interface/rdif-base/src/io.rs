use core::fmt::Display;

pub type Result<T = ()> = core::result::Result<T, Error>;

pub trait Read {
    /// Read data from the device. Returns the number of bytes read.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;
    /// Returns true if the device is ready to read.
    fn can_read(&self) -> bool;
}

pub trait Write {
    /// Write data to the device. Returns the number of bytes written.
    fn write(&mut self, buf: &[u8]) -> Result<usize>;
    /// Returns true if the device is ready to accept data.
    fn can_write(&self) -> bool;
}

impl core::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone)]
pub enum Error {
    /// Unspecified error kind.
    Other(&'static str),
    /// The operation lacked the necessary privileges to complete.
    PermissionDenied,
    /// Hardware not available.
    NotAvailable,
    /// The operation failed because a pipe was closed.
    BrokenPipe,
    /// parameter was incorrect.
    InvalidParameter { name: &'static str },
    /// Data not valid for the operation were encountered.
    InvalidData,
    /// The I/O operation's timeout expired, causing it to be canceled.
    TimedOut,
    /// This operation was interrupted.
    ///
    /// Interrupted operations can typically be retried.
    Interrupted,
    /// This operation is unsupported on this platform.
    ///
    /// This means that the operation can never succeed.
    Unsupported,
    /// An operation could not be completed, because it failed
    /// to allocate enough memory.
    OutOfMemory,
    /// An attempted write could not write any data.
    WriteZero,
}
