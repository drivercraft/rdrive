use alloc::{boxed::Box, format, string::String};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DriverError {
    #[error("fdt error: {0}")]
    Fdt(String),
    #[error("unknown driver error: {0}")]
    Unknown(String),
}

impl From<fdt_parser::FdtError<'_>> for DriverError {
    fn from(value: fdt_parser::FdtError<'_>) -> Self {
        Self::Fdt(format!("{value:?}"))
    }
}

impl From<Box<dyn core::error::Error>> for DriverError {
    fn from(value: Box<dyn core::error::Error>) -> Self {
        Self::Unknown(format!("{value:?}"))
    }
}
