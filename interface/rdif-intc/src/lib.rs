#![no_std]

extern crate alloc;

pub use alloc::{boxed::Box, vec::Vec};
use core::error::Error;

use rdif_base::custom_type;
pub use rdif_base::{DriverGeneric, DriverResult, IrqConfig, IrqId, Trigger};

custom_type!(CpuId, usize, "{:#x}");

pub type Hardware = Box<dyn Interface>;

/// Fdt 解析 `interrupts` 函数，一次解析一个`cell`
pub type FuncFdtParseConfig =
    fn(prop_interrupts_one_cell: &[u32]) -> Result<IrqConfig, Box<dyn Error>>;

/// 在中断中调用，不会被打断，视为`Sync`
pub trait InterfaceCPUNormal: Send + Sync {
    fn ack(&self) -> Option<IrqId>;
    fn eoi(&self, irq: IrqId);
}

pub trait InterfaceCPUEoiTwoStep: Send + Sync {
    fn ack(&self) -> Option<IrqId>;
    /// 降级优先级，允许中断被抢占
    fn priority_drop(&self, intid: IrqId);
    /// 中断处理完成，可继续触发中断
    fn deactivation(&self, intid: IrqId);
}

pub enum HardwareCPU {
    Normal(Box<dyn InterfaceCPUNormal>),
    EoiTwoStep(Box<dyn InterfaceCPUEoiTwoStep>),
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
