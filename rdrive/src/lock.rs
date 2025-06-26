use core::{
    any::Any,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicI64, Ordering},
};

use alloc::{
    boxed::Box,
    sync::{Arc, Weak},
};

use crate::{Descriptor, Pid, driver::Class, get_pid};

pub struct DeviceOwner {
    lock: Arc<LockInner>,
}

impl DeviceOwner {
    pub fn new<T: Class>(descriptor: Descriptor, device: T) -> Self {
        Self {
            lock: Arc::new(LockInner::new(descriptor, Box::into_raw(Box::new(device)))),
        }
    }

    pub fn weak<T: Class>(&self) -> Result<Device<T>, GetDeviceError> {
        Device::new(&self.lock)
    }

    pub fn is<T: Class>(&self) -> bool {
        unsafe { &*self.lock.ptr }.is::<T>()
    }
}

impl Drop for LockInner {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.ptr;
            let _ = Box::from_raw(ptr);
        }
    }
}

struct LockInner {
    borrowed: AtomicI64,
    ptr: *mut dyn Any,
    descriptor: Descriptor,
}

unsafe impl Send for LockInner {}
unsafe impl Sync for LockInner {}

impl LockInner {
    fn new(descriptor: Descriptor, ptr: *mut dyn Any) -> Self {
        Self {
            borrowed: AtomicI64::new(-1),
            ptr,
            descriptor,
        }
    }

    pub fn try_lock(self: &Arc<Self>, pid: Pid) -> Result<(), GetDeviceError> {
        let mut pid = pid;
        if pid.is_not_set() {
            pid = Pid::INVALID.into();
        }

        let id: usize = pid.into();

        match self.borrowed.compare_exchange(
            Pid::NOT_SET as _,
            id as _,
            Ordering::Acquire,
            Ordering::Relaxed,
        ) {
            Ok(_) => Ok(()),
            Err(old) => {
                if old as usize == Pid::INVALID {
                    Err(GetDeviceError::UsedByUnknown)
                } else {
                    let pid: Pid = (old as usize).into();
                    Err(GetDeviceError::UsedByOthers(pid))
                }
            }
        }
    }

    pub fn lock(self: &Arc<Self>) -> Result<(), GetDeviceError> {
        let pid = get_pid();
        loop {
            match self.try_lock(pid) {
                Ok(guard) => return Ok(guard),
                Err(GetDeviceError::UsedByOthers(_)) | Err(GetDeviceError::UsedByUnknown) => {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
    }
}

pub struct DeviceGuard<T> {
    lock: Arc<LockInner>,
    ptr: *mut T,
}

unsafe impl<T> Send for DeviceGuard<T> {}

impl<T> Drop for DeviceGuard<T> {
    fn drop(&mut self) {
        self.lock
            .borrowed
            .store(Pid::NOT_SET as _, Ordering::Release);
    }
}

impl<T> Deref for DeviceGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<T> DerefMut for DeviceGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ptr }
    }
}

impl<T> DeviceGuard<T> {
    pub fn descriptor(&self) -> &Descriptor {
        &self.lock.descriptor
    }
}

#[derive(Clone)]
pub struct Device<T> {
    lock: Weak<LockInner>,
    descriptor: Descriptor,
    ptr: *mut T,
}

unsafe impl<T> Send for Device<T> {}
unsafe impl<T> Sync for Device<T> {}

impl<T: Any> Device<T> {
    fn new(lock: &Arc<LockInner>) -> Result<Self, GetDeviceError> {
        let ptr = match unsafe { &*lock.ptr }.downcast_ref::<T>() {
            Some(v) => v as *const T as *mut T,
            None => return Err(GetDeviceError::TypeNotMatch),
        };

        Ok(Self {
            lock: Arc::downgrade(lock),
            descriptor: lock.descriptor.clone(),
            ptr,
        })
    }

    pub fn lock(&self) -> Result<DeviceGuard<T>, GetDeviceError> {
        let lock = self.lock.upgrade().ok_or(GetDeviceError::DeviceReleased)?;
        lock.lock()?;

        Ok(DeviceGuard {
            lock,
            ptr: self.ptr,
        })
    }
    pub fn try_lock(&self) -> Result<DeviceGuard<T>, GetDeviceError> {
        let lock = self.lock.upgrade().ok_or(GetDeviceError::DeviceReleased)?;
        lock.try_lock(get_pid())?;

        Ok(DeviceGuard {
            lock,
            ptr: self.ptr,
        })
    }

    pub fn descriptor(&self) -> &Descriptor {
        &self.descriptor
    }

    /// 强制获取设备
    ///
    /// # Safety
    /// 一般用于中断处理中
    pub unsafe fn force_use(&self) -> *mut T {
        self.ptr
    }
}

impl<T: Class> Device<T> {
    pub fn downcast<T2: 'static>(&self) -> Result<Device<T2>, GetDeviceError> {
        let lock = self.lock.upgrade().ok_or(GetDeviceError::DeviceReleased)?;

        let t2_any = unsafe { &mut *self.ptr }
            .raw_any_mut()
            .ok_or(GetDeviceError::TypeNotMatch)?;

        let t2_type = t2_any
            .downcast_mut::<T2>()
            .ok_or(GetDeviceError::TypeNotMatch)?;

        Ok(Device {
            lock: Arc::downgrade(&lock),
            descriptor: self.descriptor.clone(),
            ptr: t2_type as *mut T2,
        })
    }
}

#[derive(thiserror::Error, Debug, Clone, Copy)]
pub enum GetDeviceError {
    #[error("Used by pid: {0:?}")]
    UsedByOthers(Pid),
    #[error("Used by unknown pid")]
    UsedByUnknown,
    #[error("Device type not match")]
    TypeNotMatch,
    #[error("Device released")]
    DeviceReleased,
    #[error("Device not found")]
    NotFound,
}
