use core::{
    any::Any,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicI64, Ordering},
};

use alloc::{
    boxed::Box,
    sync::{Arc, Weak},
};
use rdif_clk::DriverGeneric;

use crate::{Descriptor, Pid, get_pid};

pub struct DeviceOwner {
    lock: Arc<LockInner>,
}

impl DeviceOwner {
    pub fn new<T: DriverGeneric + 'static>(descriptor: Descriptor, device: T) -> Self {
        Self {
            lock: Arc::new(LockInner::new(descriptor, Box::into_raw(Box::new(device)))),
        }
    }

    pub fn weak_typed<T: DriverGeneric>(&self) -> Result<DeviceWeakTyped<T>, GetDeviceError> {
        if !self.is::<T>() {
            return Err(GetDeviceError::TypeNotMatch);
        }
        Ok(DeviceWeakTyped::new(DeviceWeak::new(&self.lock)))
    }

    pub fn weak(&self) -> DeviceWeak {
        DeviceWeak::new(&self.lock)
    }

    pub fn is<T: DriverGeneric>(&self) -> bool {
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

    fn is<T: DriverGeneric>(&self) -> bool {
        unsafe { &*self.ptr }.is::<T>()
    }

    pub fn try_lock<T: DriverGeneric>(
        self: &Arc<Self>,
        pid: Pid,
        check: bool,
    ) -> Result<DeviceGuard<T>, GetDeviceError> {
        if check && !self.is::<T>() {
            return Err(GetDeviceError::TypeNotMatch);
        }
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
            Ok(_) => Ok(DeviceGuard {
                lock: self.clone(),
                mark: PhantomData,
                descriptor: &self.descriptor as *const Descriptor as *mut Descriptor,
            }),
            Err(old) => {
                let pid: Pid = (old as usize).into();
                Err(GetDeviceError::UsedByOthers(pid))
            }
        }
    }

    pub fn lock<T: DriverGeneric>(self: &Arc<Self>) -> Result<DeviceGuard<T>, GetDeviceError> {
        if !self.is::<T>() {
            return Err(GetDeviceError::TypeNotMatch);
        }
        let pid = get_pid();
        loop {
            match self.try_lock(pid, false) {
                Ok(guard) => return Ok(guard),
                Err(GetDeviceError::UsedByOthers(_)) => continue,
                Err(e) => return Err(e),
            }
        }
    }
}

pub struct DeviceGuard<T: DriverGeneric> {
    lock: Arc<LockInner>,
    descriptor: *mut Descriptor,
    mark: PhantomData<T>,
}

unsafe impl<T: DriverGeneric> Send for DeviceGuard<T> {}

impl<T: DriverGeneric> Drop for DeviceGuard<T> {
    fn drop(&mut self) {
        self.lock
            .borrowed
            .store(Pid::NOT_SET as _, Ordering::Release);
    }
}

impl<T: DriverGeneric> Deref for DeviceGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            let device = &*self.lock.ptr;
            device.downcast_ref().expect("DeviceGuard type mismatch")
        }
    }
}

impl<T: DriverGeneric> DerefMut for DeviceGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            let device = &mut *self.lock.ptr;
            device.downcast_mut().expect("DeviceGuard type mismatch")
        }
    }
}

impl<T: DriverGeneric> DeviceGuard<T> {
    pub fn descriptor(&self) -> &Descriptor {
        unsafe { &*self.descriptor }
    }
    pub(crate) fn descriptor_mut(&mut self) -> &mut Descriptor {
        unsafe { &mut *self.descriptor }
    }
}

pub struct DeviceWeak {
    lock: Weak<LockInner>,
}

impl DeviceWeak {
    fn new(lock: &Arc<LockInner>) -> Self {
        Self {
            lock: Arc::downgrade(lock),
        }
    }

    pub fn try_lock<T: DriverGeneric>(&self) -> Result<DeviceGuard<T>, GetDeviceError> {
        self.lock
            .upgrade()
            .ok_or(GetDeviceError::DeviceReleased)?
            .try_lock(get_pid(), true)
    }
    pub fn lock<T: DriverGeneric>(&self) -> Result<DeviceGuard<T>, GetDeviceError> {
        self.lock
            .upgrade()
            .ok_or(GetDeviceError::DeviceReleased)?
            .lock()
    }
}

pub struct DeviceWeakTyped<T> {
    dev: DeviceWeak,
    mark: PhantomData<T>,
}

impl<T: DriverGeneric> DeviceWeakTyped<T> {
    fn new(dev: DeviceWeak) -> Self {
        Self {
            dev,
            mark: PhantomData,
        }
    }

    pub fn lock(&self) -> Result<DeviceGuard<T>, GetDeviceError> {
        self.dev.lock()
    }
    pub fn try_lock(&self) -> Result<DeviceGuard<T>, GetDeviceError> {
        self.dev.try_lock()
    }
}

#[derive(thiserror::Error, Debug, Clone, Copy)]
pub enum GetDeviceError {
    #[error("used by pid: {0:?}")]
    UsedByOthers(Pid),
    #[error("device type not match")]
    TypeNotMatch,
    #[error("device released")]
    DeviceReleased,
    #[error("device not found")]
    NotFound,
}
