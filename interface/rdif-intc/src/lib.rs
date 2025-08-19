#![no_std]

extern crate alloc;

use alloc::string::String;

pub use rdif_base::{CpuId, DriverGeneric, KError, irq::*};

/// Fdt 解析 `interrupts` 函数，一次解析一个`cell`
pub type FuncFdtParseConfig = fn(prop_interrupts_one_cell: &[u32]) -> Result<IrqConfig, String>;

pub trait Interface: DriverGeneric {
    /// If not supported, returns None
    fn parse_dtb_fn(&self) -> Option<FuncFdtParseConfig> {
        None
    }
}
