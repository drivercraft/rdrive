#![no_std]

extern crate alloc;

pub use alloc::{boxed::Box, vec::Vec};
use core::error::Error;

use cfg_if::cfg_if;
use rdif_base::custom_type;
pub use rdif_base::{DriverGeneric, DriverResult, IrqConfig, IrqId, Trigger};

custom_type!(CpuId, usize, "{:#x}");

pub type Hardware = Box<dyn Interface>;
pub type HardwareCPU = Box<dyn InterfaceCPU>;

/// Fdt 解析 `interrupts` 函数，一次解析一个`cell`
pub type FuncFdtParseConfig =
    fn(prop_interrupts_one_cell: &[u32]) -> Result<IrqConfig, Box<dyn Error>>;

cfg_if! {
    if #[cfg(target_arch = "aarch64")]{
        pub trait InterfaceCPU: Send + Sync {
            fn setup(&self);
            fn set_eoi_mode(&self, b: bool);
            fn get_eoi_mode(&self) -> bool;
            fn ack(&self) -> Option<IrqId>;
            fn eoi(&self, intid: IrqId);
            fn dir(&self, intid: IrqId);
        }
    }else{
        /// 在中断中调用，不会被打断，视为`Sync`
        pub trait InterfaceCPU: Send + Sync {
            fn setup(&self);
            fn ack(&self) -> Option<IrqId>;
            fn eoi(&self, irq: IrqId);
        }
    }
}

pub trait Interface: DriverGeneric {
    fn cpu_interface(&self) -> HardwareCPU;
    fn irq_enable(&mut self, irq: IrqId);
    fn irq_disable(&mut self, irq: IrqId);
    fn set_priority(&mut self, irq: IrqId, priority: usize);
    fn set_trigger(&mut self, irq: IrqId, trigger: Trigger);
    fn set_target_cpu(&mut self, irq: IrqId, cpu: CpuId);
    fn capabilities(&self) -> Vec<Capability> {
        Vec::new()
    }
}

pub enum Capability {
    FdtParseConfig(FuncFdtParseConfig),
}
