#![no_std]

extern crate alloc;

pub use alloc::boxed::Box;
use rdif_base::custom_type;
pub use rdif_base::{DriverGeneric, DriverResult};

pub type Hardware = Box<dyn Interface>;

pub struct Clock {
    pub id: u32,
    pub name: Option<&'static str>,
    pub parent: Option<ClockId>,
    pub rate: u64,
    pub enabled: bool,
    pub phase: i32,
}

custom_type!(ClockId, usize, "{:#x}");

pub trait Interface: DriverGeneric {
    fn perper_enable(&mut self);

    fn get_rate(&self, id: ClockId) -> DriverResult<u64>;

    fn set_rate(&mut self, id: ClockId, rate: u64) -> DriverResult<u64>;
}
