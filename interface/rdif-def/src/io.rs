use core::fmt::Display;

pub type Result<T = ()> = core::result::Result<T, Error>;

/// Io error
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// The kind of error
    pub kind: ErrorKind,
    /// The position of the valid data
    pub success_pos: usize,
}

impl Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "success pos {}, err:{}", self.success_pos, self.kind)
    }
}

impl core::error::Error for Error {}

/// Io error kind
#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    #[error("Other error: {0}")]
    Other(&'static str),
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Hardware not available")]
    NotAvailable,
    #[error("Broken pipe")]
    BrokenPipe,
    #[error("Invalid parameter: {name}")]
    InvalidParameter { name: &'static str },
    #[error("Invalid data")]
    InvalidData,
    #[error("Timed out")]
    TimedOut,
    /// This operation was interrupted.
    ///
    /// Interrupted operations can typically be retried.
    #[error("Interrupted")]
    Interrupted,
    /// This operation is unsupported on this platform.
    ///
    /// This means that the operation can never succeed.
    #[error("Unsupported")]
    Unsupported,
    /// An operation could not be completed, because it failed
    /// to allocate enough memory.
    #[error("Out of memory")]
    OutOfMemory,
    /// An attempted write could not write any data.
    #[error("Write zero")]
    WriteZero,
}
