#![cfg_attr(not(test), no_std)]

extern crate alloc;

use as_any::AsAny;
pub use rdif_def::{CpuId, KError, custom_type, irq};

pub mod io;

pub trait DriverGeneric: Send + AsAny {
    fn open(&mut self) -> Result<(), KError>;
    fn close(&mut self) -> Result<(), KError>;
}
