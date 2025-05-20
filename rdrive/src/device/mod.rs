use core::ops::{Deref, DerefMut};

pub use descriptor::Descriptor;
pub use descriptor::DeviceId;
use rdif_base::DriverGeneric;
use rdif_base::lock::{Lock, LockGuard, LockWeak};
pub use rdif_base::lock::{LockError, PId};

pub mod block;
pub mod clk;
mod descriptor;
pub mod intc;
pub mod power;
pub mod systick;
pub mod timer;

macro_rules! define_kind {
    ($( $en:ident, $t:path; )*) => {
        pub enum HardwareKind {
            $(
                $en($t),
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
                $en(Device<$t>),
            )*
        }

        impl DeviceKind{
            pub(crate) fn open(&self)->Result<(), rdif_base::ErrorBase>{
                match self{
                    $(
                        Self::$en(d)=>d.try_borrow_by(0.into()).unwrap().open(),
                    )*
                }
            }
        }
    };
}

pub struct Empty;

impl DriverGeneric for Empty {
    fn open(&mut self) -> Result<(), rdif_base::ErrorBase> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), rdif_base::ErrorBase> {
        Ok(())
    }
}

define_kind!(
    Intc, rdif_intc::Hardware;
    Systick, rdif_systick::Hardware;
    Power, rdif_power::Hardware;
    Block, rdif_block::Hardware;
    Clk, rdif_clk::Hardware;
    SysInit, Empty;
);

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
