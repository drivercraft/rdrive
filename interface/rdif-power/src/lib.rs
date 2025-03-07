#![no_std]

extern crate alloc;

use alloc::boxed::Box;

pub use rdif_base::DriverGeneric;

pub type Hardware = Box<dyn Interface>;

pub trait Interface: DriverGeneric {
    fn shutdown(&mut self);
}
