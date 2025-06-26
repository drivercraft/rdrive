#![no_std]
#![feature(box_as_ptr)]

extern crate alloc;

use core::ptr::NonNull;
pub use fdt_parser::Phandle;

use log::warn;
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

use crate::{driver::Class, error::DriverError, probe::OnProbeError};

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
    for one in registers {
        match probe_one(one) {
            Ok(_) => {} // Successfully probed, move to the next
            Err(e) => {
                if stop_if_fail {
                    return Err(e);
                } else {
                    warn!("Probe failed for [{}]: {}", one.register.name, e);
                }
            }
        }
    }

    Ok(())
}

fn probe_one(one: &DriverRegisterData) -> Result<(), ProbeError> {
    let to_probe = edit(|manager| manager.to_unprobed(one))?;
    for to_probe in to_probe {
        match to_probe() {
            Ok(_) => {
                edit(|manager| manager.registers.set_probed(one.id));
                return Ok(());
            }
            Err(OnProbeError::NotMatch) => {
                continue; // Not a match, skip to the next probe
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }
    Ok(())
}

pub fn probe_all(stop_if_fail: bool) -> Result<(), ProbeError> {
    let unregistered = edit(|manager| manager.unregistered())?;
    probe_with(unregistered.iter(), stop_if_fail)
}

pub fn get_list<T: Class>() -> Vec<Device<T>> {
    read(|manager| manager.dev_container.devices())
}

pub fn get<T: Class>(id: DeviceId) -> Result<Device<T>, GetDeviceError> {
    read(|manager| manager.dev_container.get_typed(id))
}

pub fn get_one<T: Class>() -> Option<Device<T>> {
    read(|manager| manager.dev_container.get_one())
}

/// Macro for generating a driver module.
///
/// This macro automatically generates a driver registration module that creates a static
/// `DriverRegister` struct containing driver metadata (such as name, probe level, priority,
/// and probe types). The generated static variable is placed in the special linker section
/// `.driver.register` to be automatically discovered and registered by the driver manager
/// at runtime.
///
/// # Parameters
/// - `$i:ident`: Field identifier (e.g., `name`, `level`, `priority`, `probe_kinds`)
/// - `$t:expr`: Expression for the corresponding field value
///
/// # Generated Code
///
/// The macro generates a module containing a static `DriverRegister` that:
/// - Uses `#[link_section = ".driver.register"]` attribute to place it in a special linker section
/// - Uses `#[no_mangle]` and `#[used]` to prevent compiler optimization
/// - Contains all driver registration information
///
/// # Example
///
/// ```rust
/// use rdrive::{
///     module_driver,
///     driver::*,
///     register::FdtInfo,
///     probe::OnProbeError,
///     PlatformDevice,
/// };
///
/// struct UartDriver {}
///
/// impl DriverGeneric for UartDriver {
///     fn open(&mut self) -> Result<(), rdrive::KError> { todo!() }
///     fn close(&mut self) -> Result<(), rdrive::KError> { todo!() }
/// }
///
/// impl rdrive::driver::serial::Interface for UartDriver {
///     fn handle_irq(&mut self) { todo!() }
///     fn take_tx(&mut self) -> Option<Box<(dyn rdrive::driver::serial::io::Write + 'static)>> { todo!() }
///     fn take_rx(&mut self) -> Option<Box<(dyn rdrive::driver::serial::io::Read + 'static)>> { todo!() }
/// }
///
/// // Define probe function
/// fn probe_uart(fdt: FdtInfo<'_>, dev: PlatformDevice) -> Result<(), OnProbeError> {
///     // Implement specific device probing logic
///     dev.register_serial(UartDriver{});
///     Ok(())
/// }
///
/// // Use macro to generate driver registration module
/// module_driver! {
///     name: "UART Driver",
///     level: ProbeLevel::PostKernel,
///     priority: ProbePriority::DEFAULT,
///     probe_kinds: &[ProbeKind::Fdt {
///         compatibles: &["ns16550a", "arm,pl011"],
///         // Use `probe_uart` above; this usage is because doctests cannot find the parent module.
///         on_probe: |fdt, dev|{
///             Ok(())
///         },
///     }],
/// }
/// ```
///
/// # Notes
///
/// - This macro can only be used once per driver module
/// - The generated module name is automatically derived from the driver name
/// - All fields must be properly set, especially the `probe_kinds` array
/// - Probe functions must implement the correct signature and error handling
#[macro_export]
macro_rules! module_driver {
    (
        $($i:ident : $t:expr),+,
    ) => {
        /// Auto-generated driver registration module.
        #[allow(unused)]
        $crate::__mod_maker!{
            pub mod some {
                use super::*;
                use $crate::register::*;

                /// Static instance of driver registration information.
                ///
                /// This static variable is placed in the `.driver.register` linker section
                /// so that the driver manager can automatically discover and load it during
                /// system startup.
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
