use alloc::{
    boxed::Box,
    string::{String, ToString},
    vec::Vec,
};
use core::error::Error;
use enum_dispatch::enum_dispatch;

use fdt_parser::FdtError;

use crate::{Platform, error::DriverError, register::DriverRegisterData};

pub mod fdt;
pub mod pci;

#[derive(thiserror::Error, Debug)]
pub enum ProbeError {
    #[error("probe `{name}` fail: irq chip not init")]
    IrqNotInit { name: String },
    #[error("fdt parse error: {0}")]
    Fdt(String),
    #[error("on probe error: {0}")]
    OnProbe(#[from] OnProbeError),
    #[error("open device fail")]
    OpenFail(#[from] rdif_base::KError),
}

impl From<FdtError<'_>> for ProbeError {
    fn from(value: FdtError) -> Self {
        Self::Fdt(format!("{value:?}"))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum OnProbeError {
    #[error("probe not match")]
    NotMatch,
    #[error("kerror: {0}")]
    KError(#[from] rdif_base::KError),
    #[error("other error: {0}")]
    Other(#[from] Box<dyn Error>),
    #[error("fdt parse error: {0}")]
    Fdt(String),
}

impl From<FdtError<'_>> for OnProbeError {
    fn from(value: FdtError) -> Self {
        Self::Fdt(format!("{value:?}"))
    }
}

impl OnProbeError {
    pub fn other(msg: impl AsRef<str>) -> Self {
        Self::Other(msg.as_ref().to_string().into())
    }
}

#[enum_dispatch]
pub(crate) enum EnumSystem {
    Fdt(fdt::System),
}

#[enum_dispatch(EnumSystem)]
pub(crate) trait EnumSystemTrait {
    fn to_unprobed(
        &mut self,
        register: &DriverRegisterData,
    ) -> Result<Vec<ToProbeFunc>, ProbeError>;
}

impl EnumSystem {
    pub fn new(platform: Platform) -> Result<Self, DriverError> {
        Ok(match platform {
            Platform::Fdt { addr } => Self::Fdt(fdt::System::new(addr)?),
        })
    }
}

pub(crate) type ToProbeFunc = Box<dyn FnOnce() -> Result<(), OnProbeError>>;
