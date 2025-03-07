#![no_std]

extern crate alloc;

use alloc::boxed::Box;

pub use rdif_base::{DriverGeneric, DriverResult, IrqConfig, IrqId, Trigger};

pub type Hardware = Box<dyn Interface>;

pub trait Interface: Send {
    fn shutdown(&mut self);
}
