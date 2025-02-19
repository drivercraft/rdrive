use alloc::{collections::BTreeMap, vec::Vec};
use core::ops::{Deref, DerefMut};

pub use descriptor::Descriptor;
pub use descriptor::DeviceId;
use rdif_base::lock::{Lock, LockGuard, LockWeak};
pub use rdif_base::lock::{LockError, PId};

mod descriptor;
pub mod intc;
pub mod timer;

pub struct Device<T> {
    pub descriptor: Descriptor,
    driver: Lock<T>,
}

impl<T> Device<T> {
    pub fn new(descriptor: Descriptor, driver: T) -> Self {
        Self {
            descriptor,
            driver: Lock::new(driver),
        }
    }

    pub fn try_borrow_by(&self, pid: PId) -> Result<DeviceGuard<T>, LockError> {
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

impl<T> DeviceWeak<T> {
    pub fn upgrade(&self) -> Option<Device<T>> {
        self.driver.upgrade().map(|d| Device {
            descriptor: self.descriptor.clone(),
            driver: d,
        })
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

pub struct Container<T> {
    data: BTreeMap<DeviceId, Device<T>>,
}

impl<T> Container<T> {
    pub const fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, dev: Device<T>) {
        self.data.insert(dev.descriptor.device_id, dev);
    }

    pub fn get(&self, id: DeviceId) -> Option<DeviceWeak<T>> {
        self.data.get(&id).map(|o| o.weak())
    }

    pub fn all(&self) -> Vec<(DeviceId, DeviceWeak<T>)> {
        self.data.iter().map(|(i, o)| (*i, o.weak())).collect()
    }
}

impl<T> Default for Container<T> {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

pub struct ProbedDevice {
    pub register_id: usize,
    pub kind: DeviceKind,
}

pub enum DeviceKind {
    Intc(Device<intc::Hardware>),
    Timer(Device<timer::Hardware>),
}
