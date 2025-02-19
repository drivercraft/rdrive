#![cfg_attr(not(test), no_std)]

extern crate alloc;

#[macro_use]
mod _macro;

pub mod io;
pub mod lock;

pub type DriverResult<T = ()> = core::result::Result<T, alloc::boxed::Box<dyn core::error::Error>>;

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
