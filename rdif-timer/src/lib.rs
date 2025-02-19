#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::boxed::Box;

pub use rdif_base::{DriverGeneric, DriverResult, IrqConfig, IrqId, Trigger};

pub type Hardware = Box<dyn Interface>;
pub type HardwareCPU = Box<dyn InterfaceCPU>;

pub trait Interface: Send {
    fn get_current_cpu(&mut self) -> Box<dyn InterfaceCPU>;
}

pub trait InterfaceCPU: DriverGeneric + Sync {
    fn set_timeval(&mut self, ticks: u64);
    fn current_ticks(&self) -> u64;
    fn tick_hz(&self) -> u64;
    fn set_irq_enable(&mut self, enable: bool);
    fn get_irq_status(&self) -> bool;
    fn irq(&self) -> IrqConfig;
}
