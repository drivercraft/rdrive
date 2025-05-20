#![no_std]

extern crate alloc;

pub use rdif_base::{DriverGeneric, DriverResult};
pub use alloc::boxed::Box;

pub type Hardware = Box<dyn CruDriver>;

pub struct DtClockSpec {
    pub node: u32,
    pub args: &'static [u32],
}

pub struct Clock {
    pub id: u32,
    pub name: Option<&'static str>,
    pub parent: Option<u32>,
    pub rate: u64,
    pub enabled: bool,
    pub phase: i32,
}

pub trait CruDriver: DriverGeneric {
    fn get_rate(&self, id: u32) -> DriverResult<u64>;

    fn set_rate(&mut self, id: u32, rate: u64) -> DriverResult<u64>;
}