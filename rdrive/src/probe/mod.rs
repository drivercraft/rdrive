use core::ptr::NonNull;

use crate::{Descriptor, DriverInfoKind};

pub(crate) mod fdt;

pub enum ProbeData {
    Fdt(fdt::ProbeData),
}

impl Default for ProbeData {
    fn default() -> Self {
        Self::Fdt(fdt::ProbeData::new(NonNull::dangling()))
    }
}

impl From<DriverInfoKind> for ProbeData {
    fn from(value: DriverInfoKind) -> Self {
        match value {
            DriverInfoKind::Fdt { addr } => ProbeData::Fdt(fdt::ProbeData::new(addr)),
        }
    }
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
