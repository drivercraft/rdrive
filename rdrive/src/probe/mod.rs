use alloc::{boxed::Box, format, string::String};
use core::{error::Error, ptr::NonNull};

use fdt_parser::FdtError;

use crate::{Descriptor, DeviceKind, DriverInfoKind};

pub mod fdt;

#[derive(thiserror::Error, Debug)]
pub enum ProbeError {
    #[error("probe `{name}` fail: irq chip not init")]
    IrqNotInit { name: String },
    #[error("fdt parse error: {0}")]
    Fdt(String),
    #[error("on probe error: {0}")]
    OnProbe(Box<dyn Error>),
}

impl From<FdtError<'_>> for ProbeError {
    fn from(value: FdtError) -> Self {
        Self::Fdt(format!("{value:?}"))
    }
}

pub enum EnumSystem {
    Fdt(fdt::ProbeFunc),
}

impl Default for EnumSystem {
    fn default() -> Self {
        Self::Fdt(fdt::ProbeFunc::new(NonNull::dangling()))
    }
}

impl From<DriverInfoKind> for EnumSystem {
    fn from(value: DriverInfoKind) -> Self {
        match value {
            DriverInfoKind::Fdt { addr } => EnumSystem::Fdt(fdt::ProbeFunc::new(addr)),
        }
    }
}

pub struct ProbedDevice {
    pub register_id: usize,
    pub descriptor: Descriptor,
    pub dev: DeviceKind,
}
