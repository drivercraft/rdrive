use crate::DriverInfoKind;

pub(crate) mod fdt;

pub enum ProbeData {
    Fdt(fdt::ProbeData),
    Static,
}

impl Default for ProbeData {
    fn default() -> Self {
        Self::Static
    }
}

impl From<DriverInfoKind> for ProbeData {
    fn from(value: DriverInfoKind) -> Self {
        match value {
            DriverInfoKind::Fdt { addr } => ProbeData::Fdt(fdt::ProbeData::new(addr)),
            DriverInfoKind::Static => ProbeData::Static,
        }
    }
}
