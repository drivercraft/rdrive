#![no_std]
#![feature(box_as_ptr)]

extern crate alloc;

use core::ptr::NonNull;
pub use fdt_parser::Phandle;

use register::{DriverRegister, DriverRegisterData, ProbeLevel};
use spin::Mutex;

mod descriptor;
pub mod driver;
pub mod error;
mod id;
mod lock;
mod manager;
mod osal;

pub mod probe;
pub mod register;

pub use descriptor::*;
pub use driver::PlatformDevice;
pub use lock::*;
pub use manager::*;
pub use osal::*;
pub use probe::ProbeError;
pub use rdif_base::{DriverGeneric, KError, irq::IrqId};
pub use rdrive_macros::*;

use crate::error::DriverError;

static MANAGER: Mutex<Option<Manager>> = Mutex::new(None);

#[derive(Debug, Clone)]
pub enum Platform {
    Fdt { addr: NonNull<u8> },
}

unsafe impl Send for Platform {}

pub fn init(platform: Platform) -> Result<(), DriverError> {
    let mut g = MANAGER.lock();
    if g.is_none() {
        g.replace(Manager::new(platform)?);
    }
    Ok(())
}

pub(crate) fn edit<F, T>(f: F) -> T
where
    F: FnOnce(&mut Manager) -> T,
{
    let mut g = MANAGER.lock();
    f(g.as_mut().expect("manager not init"))
}

pub(crate) fn read<F, T>(f: F) -> T
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
    let unregistered = edit(|manager| manager.unregistered())?;

    let ls = unregistered
        .iter()
        .filter(|one| matches!(one.register.level, ProbeLevel::PreKernel));

    probe_with(ls, true)?;

    Ok(())
}

fn probe_with<'a>(
    registers: impl Iterator<Item = &'a DriverRegisterData>,
    stop_if_fail: bool,
) -> Result<(), ProbeError> {
    macro_rules! handle_error {
        ($e: expr, $m: expr) => {
            if stop_if_fail {
                $e?
            } else {
                match $e {
                    Ok(v) => v,
                    Err(e) => {
                        log::warn!("{}: {}", $m, e);
                        continue;
                    }
                }
            }
        };
    }

    for one in registers {
        let to_probe = edit(|manager| manager.to_unprobed(one))?;

        if let Some(to_probe) = to_probe {
            handle_error!(to_probe(), "probe fail");
        }
    }

    Ok(())
}

pub fn probe_all(stop_if_fail: bool) -> Result<(), ProbeError> {
    let unregistered = edit(|manager| manager.unregistered())?;
    probe_with(unregistered.iter(), stop_if_fail)
}

pub fn get_list<T: DriverGeneric>() -> Vec<Device<T>> {
    read(|manager| manager.dev_container.devices())
}

pub fn get<T: DriverGeneric>(id: DeviceId) -> Result<Device<T>, GetDeviceError> {
    read(|manager| manager.dev_container.get_typed(id))
}

pub fn get_raw(id: DeviceId) -> Option<DeviceWeak> {
    read(|manager| manager.dev_container.get(id))
}

pub fn get_one<T: DriverGeneric>() -> Option<Device<T>> {
    read(|manager| manager.dev_container.get_one())
}

#[macro_export]
macro_rules! module_driver {
    (
        $($i:ident : $t:expr),+,
    ) => {
        /// Generate a module for the driver.
        #[allow(unused)]
        $crate::__mod_maker!{
            pub mod some {
                use super::*;
                use $crate::register::*;

                ///  Register the driver.
                #[unsafe(link_section = ".driver.register")]
                #[unsafe(no_mangle)]
                #[link(used)]
                pub static DRIVER: DriverRegister = DriverRegister {
                    $($i : $t),+
                };
            }
        }
    };
}
