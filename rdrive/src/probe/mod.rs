use alloc::{string::String, vec::Vec};
use core::ptr::NonNull;

use crate::{Descriptor, DeviceId, DriverInfoKind, intc::IrqConfig};

pub(crate) mod fdt;

#[derive(thiserror::Error, Debug)]
pub enum ProbeError {
    #[error("probe `{name}` fail: irq chip not init")]
    IrqNotInit { name: String },
    #[error("fdt parse error: {0}")]
    Fdt(String),
}

pub enum ProbeKind {
    Fdt(fdt::ProbeFunc),
}

impl Default for ProbeKind {
    fn default() -> Self {
        Self::Fdt(fdt::ProbeFunc::new(NonNull::dangling()))
    }
}

impl From<DriverInfoKind> for ProbeKind {
    fn from(value: DriverInfoKind) -> Self {
        match value {
            DriverInfoKind::Fdt { addr } => ProbeKind::Fdt(fdt::ProbeFunc::new(addr)),
        }
    }
}

pub struct ProbeDevInfo {
    pub irqs: Vec<IrqConfig>,
    pub irq_parent: Option<DeviceId>,
}

pub enum HardwareKind {
    Intc(rdif_intc::Hardware),
    Timer(rdif_timer::Hardware),
    Power(rdif_power::Hardware),
}

pub struct ProbedDevice {
    pub register_id: usize,
    pub descriptor: Descriptor,
    pub dev: HardwareKind,
}
