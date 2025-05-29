#![no_std]
use core::any::Any;

pub use rdif_def::KError;

extern crate alloc;
#[macro_use]
extern crate rdif_def;

pub mod io;
pub mod lock;

pub trait DriverGeneric: Send + Any {
    fn open(&mut self) -> Result<(), KError>;
    fn close(&mut self) -> Result<(), KError>;
}