pub type Result<T = ()> = core::result::Result<T, Error>;

/// Io error
#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum Error {
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
