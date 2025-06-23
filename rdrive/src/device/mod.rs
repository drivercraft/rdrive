use core::ops::{Deref, DerefMut};

use alloc::collections::btree_map::BTreeMap;
pub use descriptor::Descriptor;
pub use descriptor::DeviceId;
use paste::paste;
use rdif_base::DriverGeneric;
use rdif_base::lock::{Lock, LockGuard, LockWeak};
pub use rdif_base::lock::{LockError, PId};
mod descriptor;
mod lock;

pub use lock::*;

macro_rules! define_kind {
    ($( $en:ident, )*) => {
        paste!{
            pub enum HardwareKind {
                $(
                    $en([<$en:lower>]::Boxed),
                )*
            }

            impl HardwareKind{
                pub fn to_device(self, desc: Descriptor)->DeviceKind{
                    match self{
                        $(
                            Self::$en(d)=> DeviceKind::$en( Device::new(desc,d)),
                        )*
                    }
                }
            }
            pub enum DeviceKind {
                $(
                    $en([<$en:lower>]::Device),
                )*
            }

            impl DeviceKind{
                pub(crate) fn open(&self)->Result<(), rdif_base::KError>{
                    match self{
                        $(
                            Self::$en(d)=>d.try_borrow_by(0.into()).unwrap().open(),
                        )*
                    }
                }
            }

            $(
                pub mod [<$en:lower>]{
                    pub use [<rdif_ $en:lower>]::*;

                    pub type Boxed = alloc::boxed::Box<dyn Interface>;
                    pub type Device = super::Device<Boxed>;
                    pub type Weak = super::DeviceWeak<Boxed>;
                }
            )*
        }
    };
}

pub struct Empty;

impl DriverGeneric for Empty {
    fn open(&mut self) -> Result<(), rdif_base::KError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), rdif_base::KError> {
        Ok(())
    }
}

define_kind!(Intc, Systick, Power, Block, Clk, Serial,);

pub struct Device<T> {
    pub descriptor: Descriptor,
    driver: Lock<T>,
}

impl<T: 'static> Device<T> {
    pub fn new(descriptor: Descriptor, driver: T) -> Self {
        Self {
            descriptor,
            driver: Lock::new(driver),
        }
    }

    pub fn try_borrow_by(&self, pid: PId) -> Result<DeviceGuard<T>, DeviceError> {
        let g = self.driver.try_borrow(pid)?;
        Ok(DeviceGuard {
            descriptor: self.descriptor.clone(),
            lock: g,
        })
    }

    pub fn weak(&self) -> DeviceWeak<T> {
        DeviceWeak {
            descriptor: self.descriptor.clone(),
            driver: self.driver.weak(),
        }
    }

    pub fn spin_try_borrow_by(&self, pid: PId) -> DeviceGuard<T> {
        loop {
            match self.try_borrow_by(pid) {
                Ok(g) => {
                    return g;
                }
                Err(_) => continue,
            }
        }
    }

    /// 强制获取设备
    ///
    /// # Safety
    /// 一般用于中断处理中
    pub unsafe fn force_use(&self) -> *mut T {
        unsafe { self.driver.force_use() }
    }
}

impl<T: Sync + Send> Deref for Device<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.driver
    }
}

pub struct DeviceWeak<T> {
    pub descriptor: Descriptor,
    driver: LockWeak<T>,
}
impl<T> Clone for DeviceWeak<T> {
    fn clone(&self) -> Self {
        Self {
            descriptor: self.descriptor.clone(),
            driver: self.driver.clone(),
        }
    }
}

impl<T> DeviceWeak<T> {
    pub fn upgrade(&self) -> Option<Device<T>> {
        self.driver.upgrade().map(|d| Device {
            descriptor: self.descriptor.clone(),
            driver: d,
        })
    }

    pub fn try_borrow_by(&self, pid: PId) -> Result<DeviceGuard<T>, DeviceError> {
        let one = self.upgrade().ok_or(DeviceError::Droped)?;
        let g = one.driver.try_borrow(pid)?;
        Ok(DeviceGuard {
            descriptor: one.descriptor.clone(),
            lock: g,
        })
    }

    pub fn spin_try_borrow_by(&self, pid: PId) -> Result<DeviceGuard<T>, DeviceError> {
        loop {
            match self.try_borrow_by(pid) {
                Ok(g) => {
                    return Ok(g);
                }
                Err(e) => {
                    if let DeviceError::UsedByOthers(_) = e {
                        continue;
                    } else {
                        return Err(e);
                    }
                }
            }
        }
    }
}

pub struct DeviceGuard<T> {
    pub descriptor: Descriptor,
    lock: LockGuard<T>,
}

impl<T> Deref for DeviceGuard<T> {
    type Target = LockGuard<T>;

    fn deref(&self) -> &Self::Target {
        &self.lock
    }
}

impl<T> DerefMut for DeviceGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.lock
    }
}

#[derive(thiserror::Error, Debug, Clone, Copy)]
pub enum DeviceError {
    #[error("used by pid: {0:?}")]
    UsedByOthers(PId),
    #[error("droped")]
    Droped,
}

impl From<LockError> for DeviceError {
    fn from(value: LockError) -> Self {
        match value {
            LockError::UsedByOthers(pid) => Self::UsedByOthers(pid),
            LockError::DeviceReleased => Self::Droped,
        }
    }
}

pub(crate) struct DeviceContainer {
    devices: BTreeMap<DeviceId, DeviceOwner>,
}

impl DeviceContainer {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
        }
    }

    pub fn insert<T: DriverGeneric + 'static>(&mut self, id: DeviceId, device: T) {
        self.devices.insert(id, DeviceOwner::new(device));
    }

    pub fn get_typed<T: DriverGeneric>(
        &self,
        id: DeviceId,
    ) -> Result<DeviceWeakTyped<T>, GetDeviceError> {
        let dev = self.devices.get(&id).ok_or(GetDeviceError::NotFound)?;

        dev.weak_typed()
    }

    pub fn get(&self, id: DeviceId) -> Option<lock::DeviceWeak> {
        let dev = self.devices.get(&id)?;
        Some(dev.weak())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum GetDeviceError {
    #[error("device not found")]
    NotFound,
    #[error("device type not match")]
    TypeNotMatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_container() {
        let mut container = DeviceContainer::new();
        let id = DeviceId::new();
        container.insert(id, Empty);
        let weak = container.get_typed::<Empty>(id).unwrap();

        {
            let mut device = weak.lock().unwrap();

            assert!(device.open().is_ok());
            assert!(device.close().is_ok());
        }

        {
            let mut device = weak.lock().unwrap();

            assert!(device.open().is_ok());
            assert!(device.close().is_ok());
        }
    }
}
