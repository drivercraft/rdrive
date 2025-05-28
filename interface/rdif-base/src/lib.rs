#![no_std]
use core::any::Any;

#[macro_use]
mod _macro;

pub mod io;
#[cfg(feature = "alloc")]
pub mod lock;

#[derive(thiserror::Error, Debug, Clone, PartialEq, Eq)]
pub enum ErrorBase {
    #[error("IO error")]
    Io,
    #[error("No memory")]
    NoMem,
    #[error("Try Again")]
    Again,
    #[error("Busy")]
    Busy,
    #[error("Bad Address: {0:#x}")]
    BadAddr(usize),
    #[error("Invalid Argument `{name}`")]
    InvalidArg { name: &'static str },
}

pub trait DriverGeneric: Send + Any {
    fn open(&mut self) -> Result<(), ErrorBase>;
    fn close(&mut self) -> Result<(), ErrorBase>;
}

custom_type!(IrqId, usize, "{:#x}");

/// The trigger configuration for an interrupt.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Trigger {
    EdgeBoth,
    EdgeRising,
    EdgeFailling,
    LevelHigh,
    LevelLow,
}

#[derive(Debug, Clone)]
pub struct IrqConfig {
    pub irq: IrqId,
    pub trigger: Trigger,
}
