#![cfg_attr(not(test), no_std)]

extern crate alloc;

use alloc::boxed::Box;
use core::error::Error;

use rdif_base::custom_type;
pub use rdif_base::{DriverGeneric, DriverResult, IrqConfig, IrqId, Trigger};

custom_type!(CpuId, usize, "{:#x}");

pub type Hardware = Box<dyn Interface>;
pub type HardwareCPU = Box<dyn InterfaceCPU>;

/// Fdt 解析 `interrupts` 函数，一次解析一个`cell`
pub type FdtParseConfigFn =
    fn(prop_interrupts_one_cell: &[u32]) -> Result<IrqConfig, Box<dyn Error>>;

pub struct FdtProbeInfo {
    pub hardware: Hardware,
    pub fdt_parse_config_fn: FdtParseConfigFn,
}

/// 在中断中调用，不会被打断，视为`Sync`
pub trait InterfaceCPU: Send + Sync {
    fn get_and_acknowledge_interrupt(&self) -> Option<IrqId>;
    fn end_interrupt(&self, irq: IrqId);
}

pub trait Interface: DriverGeneric {
    fn current_cpu_setup(&self) -> HardwareCPU;
    fn irq_enable(&mut self, irq: IrqId);
    fn irq_disable(&mut self, irq: IrqId);
    fn set_priority(&mut self, irq: IrqId, priority: usize);
    fn set_trigger(&mut self, irq: IrqId, trigger: Trigger);
    fn set_target_cpu(&mut self, irq: IrqId, cpu: CpuId);
}
