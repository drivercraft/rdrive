#![no_std]

extern crate alloc;

use core::ptr::NonNull;
pub use fdt_parser::Phandle;

use register::DriverRegister;
use spin::Mutex;

mod device;
pub mod error;
mod id;
mod manager;
pub mod probe;
pub mod register;
pub use device::*;
pub use manager::*;
pub use probe::ProbeError;
pub use rdif_base::{DriverGeneric, DriverResult, IrqId, io};

static MANAGER: Mutex<Option<Manager>> = Mutex::new(None);

#[derive(Debug, Clone)]
pub enum DriverInfoKind {
    Fdt { addr: NonNull<u8> },
}

unsafe impl Send for DriverInfoKind {}

pub fn init(probe_kind: DriverInfoKind) {
    MANAGER.lock().replace(Manager::new(probe_kind));
}

pub fn edit<F, T>(f: F) -> T
where
    F: FnOnce(&mut Manager) -> T,
{
    let mut g = MANAGER.lock();
    f(g.as_mut().expect("manager not init"))
}

pub fn read<F, T>(f: F) -> T
where
    F: FnOnce(&Manager) -> T,
{
    let g = MANAGER.lock();
    f(g.as_ref().expect("manager not init"))
}

pub fn register_add(register: DriverRegister) {
    edit(|manager| manager.registers.add(register));
}

pub fn register_append(registers: &[DriverRegister]) {
    edit(|manager| manager.registers.append(registers))
}

pub fn probe_pre_kernel() -> Result<(), ProbeError> {
    edit(|manager| manager.probe_pre_kernel())
}

pub fn probe() -> Result<(), ProbeError> {
    edit(|manager| manager.probe())
}

#[macro_export]
macro_rules! dev_list {
    ($k: ident) => {
        $crate::read(|manager| {
            extern crate alloc;

            manager
                .dev_map
                .iter()
                .filter_map(|(_, v)| {
                    if let $crate::DeviceKind::$k(dev) = v {
                        Some(dev.weak())
                    } else {
                        None
                    }
                })
                .collect::<alloc::vec::Vec<_>>()
        })
    };
}
#[macro_export]
macro_rules! get_dev {
    ($k:ident) => {
        $crate::read(|m| {
            m.dev_map
                .iter()
                .filter_map(|(_, v)| {
                    if let $crate::DeviceKind::$k(dev) = v {
                        Some(dev.weak())
                    } else {
                        None
                    }
                })
                .next()
        })
    };
    ($id:expr, $k:ident) => {
        $crate::read(|m| {
            let dev = m.dev_map.get(&$id)?;
            if let $crate::DeviceKind::$k(dev) = dev {
                Some(dev.weak())
            } else {
                None
            }
        })
    };
}
