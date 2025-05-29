#![no_std]

pub use rdif_base::{DriverGeneric, KError};

pub trait Interface: DriverGeneric {
    fn shutdown(&mut self);
}
