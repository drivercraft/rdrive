#![no_std]

extern crate alloc;

use core::error::Error;

pub use alloc::boxed::Box;
pub use rdif_base::DriverGeneric;
use rdif_base::custom_type;

pub type Hardware = Box<dyn Interface>;

custom_type!(ClockId, usize, "{:#x}");

pub trait Interface: DriverGeneric {
    fn perper_enable(&mut self);

    fn get_rate(&self, id: ClockId) -> Result<u64, Box<dyn Error>>;

    fn set_rate(&mut self, id: ClockId, rate: u64) -> Result<(), Box<dyn Error>>;
}
