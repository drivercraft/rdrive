#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use core::error::Error;

pub use rdif_base::{CpuId, DriverGeneric, KError, irq::*};

/// CPU local interrupt controller interface
pub mod local {
    use super::*;

    /// Boxed interface
    pub type Boxed = Box<dyn Interface>;

    pub trait Interface: DriverGeneric + Sync {
        fn ack(&self) -> Option<IrqId>;
        fn eoi(&self, irq: IrqId);

        cfg_if::cfg_if! {
            if #[cfg(target_arch = "aarch64")]{
                fn dir(&self, intid: IrqId);
                fn set_eoi_mode(&self, b: bool);
                fn get_eoi_mode(&self) -> bool;
            }
        }

        fn capability(&self) -> Capability;
    }

    /// controller capability
    pub enum Capability {
        None,
        /// Local interface can config local irq.
        ConfigLocalIrq(cap::BoxedConfigLocalIrq),
    }

    pub mod cap {
        use super::*;

        pub type BoxedConfigLocalIrq = Box<dyn ConfigLocalIrq>;

        /// Local interface can config local irq.
        pub trait ConfigLocalIrq: Send + Sync {
            fn irq_enable(&self, irq: IrqId) -> Result<(), IntcError>;
            fn irq_disable(&self, irq: IrqId) -> Result<(), IntcError>;
            fn set_priority(&self, irq: IrqId, priority: usize) -> Result<(), IntcError>;
            fn set_trigger(&self, irq: IrqId, trigger: Trigger) -> Result<(), IntcError>;
        }
    }
}

/// Fdt 解析 `interrupts` 函数，一次解析一个`cell`
pub type FuncFdtParseConfig =
    fn(prop_interrupts_one_cell: &[u32]) -> Result<IrqConfig, Box<dyn Error>>;

pub trait Interface: DriverGeneric {
    fn irq_enable(&mut self, irq: IrqId) -> Result<(), IntcError>;
    fn irq_disable(&mut self, irq: IrqId) -> Result<(), IntcError>;
    fn set_priority(&mut self, irq: IrqId, priority: usize) -> Result<(), IntcError>;
    fn set_trigger(&mut self, irq: IrqId, trigger: Trigger) -> Result<(), IntcError>;
    fn set_target_cpu(&mut self, irq: IrqId, cpu: CpuId) -> Result<(), IntcError>;

    /// Get CPU local interrupt controller, return None if not supported
    fn cpu_local(&self) -> Option<local::Boxed>;
    /// If not supported, returns None
    fn parse_dtb_fn(&self) -> Option<FuncFdtParseConfig> {
        None
    }
}

#[derive(thiserror::Error, Debug)]
pub enum IntcError {
    #[error("irq `{id:?}` not compatible")]
    IrqIdNotCompatible { id: IrqId },
    #[error("not support")]
    NotSupport,
    #[error("other error: {0}")]
    Other(Box<dyn Error>),
}
