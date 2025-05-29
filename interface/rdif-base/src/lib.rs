#![cfg_attr(not(test), no_std)]

use core::any::Any;

extern crate alloc;
#[macro_use]
extern crate rdif_def;

pub use rdif_def::{CpuId, KError, custom_type, irq};

pub mod io;
pub mod lock;

pub trait DriverGeneric: Send + Any {
    fn open(&mut self) -> Result<(), KError>;
    fn close(&mut self) -> Result<(), KError>;
}
