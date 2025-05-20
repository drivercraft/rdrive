#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use core::error::Error;

#[macro_use]
mod _macro;

pub mod io;
#[cfg(feature = "alloc")]
pub mod lock;

pub trait DriverGeneric: Send {
    fn open(&mut self) -> Result<(), Box<dyn Error>>;
    fn close(&mut self) -> Result<(), Box<dyn Error>>;
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
