#![no_std]

extern crate alloc;

pub use alloc::boxed::Box;
pub use rdif_base::DriverGeneric;
pub use rdif_base::ErrorBase;
use rdif_base::custom_type;

pub type Hardware = Box<dyn Interface>;

custom_type!(ClockId, usize, "{:#x}");

pub trait Interface: DriverGeneric {
    fn perper_enable(&mut self);

    fn get_rate(&self, id: ClockId) -> Result<u64, ErrorBase>;

    fn set_rate(&mut self, id: ClockId, rate: u64) -> Result<(), ErrorBase>;
}
