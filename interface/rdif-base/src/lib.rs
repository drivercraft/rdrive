#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

#[macro_use]
mod _macro;

pub mod io;
#[cfg(feature = "alloc")]
pub mod lock;

#[derive(Debug, Clone, Copy)]
pub enum Error {
    NoDev,
    InvalidIo,
    Busy,
    InvalidArgument,
    NoMemory,
    Timeout,
}

pub type DriverResult<T = ()> = core::result::Result<T, Error>;

pub trait DriverGeneric: Send {
    fn open(&mut self) -> DriverResult;
    fn close(&mut self) -> DriverResult;
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
