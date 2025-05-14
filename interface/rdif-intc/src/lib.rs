#![no_std]

extern crate alloc;

pub use alloc::{boxed::Box, vec::Vec};
use core::{error::Error, fmt::Display};

use cfg_if::cfg_if;
use rdif_base::custom_type;
pub use rdif_base::{DriverGeneric, DriverResult, IrqConfig, IrqId, Trigger};

custom_type!(CpuId, usize, "{:#x}");

pub type Hardware = Box<dyn Interface>;
pub type BoxCPU = Box<dyn InterfaceCPU>;
pub type BoxCPUCapLocalIrq = Box<dyn CPUCapLocalIrq>;

/// Fdt 解析 `interrupts` 函数，一次解析一个`cell`
pub type FuncFdtParseConfig =
    fn(prop_interrupts_one_cell: &[u32]) -> Result<IrqConfig, Box<dyn Error>>;

pub trait CPUCapLocalIrq: Send + Sync {
    fn irq_enable(&self, irq: IrqId) -> Result<(), IntcError>;
    fn irq_disable(&self, irq: IrqId) -> Result<(), IntcError>;
    fn set_priority(&self, irq: IrqId, priority: usize) -> Result<(), IntcError>;
    fn set_trigger(&self, irq: IrqId, trigger: Trigger) -> Result<(), IntcError>;
}

pub trait InterfaceCPU: Send + Sync {
    fn setup(&self);
    fn ack(&self) -> Option<IrqId>;
    fn eoi(&self, irq: IrqId);

    cfg_if! {
        if #[cfg(target_arch = "aarch64")]{
            fn dir(&self, intid: IrqId);
            fn set_eoi_mode(&self, b: bool);
            fn get_eoi_mode(&self) -> bool;
        }
    }

    fn capability(&self) -> CPUCapability;
}

pub trait Interface: DriverGeneric {
    fn cpu_interface(&self) -> BoxCPU;

    fn irq_enable(&mut self, irq: IrqId) -> Result<(), IntcError>;
    fn irq_disable(&mut self, irq: IrqId) -> Result<(), IntcError>;
    fn set_priority(&mut self, irq: IrqId, priority: usize) -> Result<(), IntcError>;
    fn set_trigger(&mut self, irq: IrqId, trigger: Trigger) -> Result<(), IntcError>;
    fn set_target_cpu(&mut self, irq: IrqId, cpu: CpuId) -> Result<(), IntcError>;
    fn capabilities(&self) -> Vec<Capability> {
        Vec::new()
    }
}

pub enum Capability {
    FdtParseConfig(FuncFdtParseConfig),
}

pub enum CPUCapability {
    None,
    LocalIrq(BoxCPUCapLocalIrq),
}

#[derive(Debug, Clone, Copy)]
pub enum IntcError {
    IrqIdNotCompatible { id: IrqId },
    NotSupport,
}
impl Display for IntcError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for IntcError {}
