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

use crate::{GetDeviceError, Pid, get_pid};

pub struct DeviceOwner {
    lock: Arc<LockInner>,
}

impl DeviceOwner {
    pub fn new<T: DriverGeneric + 'static>(device: T) -> Self {
        Self {
            lock: Arc::new(LockInner::new(Box::into_raw(Box::new(device)))),
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
}

unsafe impl Send for LockInner {}
unsafe impl Sync for LockInner {}

impl LockInner {
    fn new(ptr: *mut dyn Any) -> Self {
        Self {
            borrowed: AtomicI64::new(-1),
            ptr,
        }
    }

    fn is<T: DriverGeneric>(&self) -> bool {
        unsafe { &*self.ptr }.is::<T>()
    }

    pub fn try_lock<T: DriverGeneric>(
        self: &Arc<Self>,
        pid: Pid,
        check: bool,
    ) -> Result<DeviceGuard<T>, LockError> {
        if check && !self.is::<T>() {
            return Err(LockError::TypeNotMatch);
        }

        let id: usize = pid.into();

        match self
            .borrowed
            .compare_exchange(-1, id as _, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(_) => Ok(DeviceGuard {
                lock: self.clone(),
                mark: PhantomData,
            }),
            Err(old) => {
                let pid: Pid = (old as usize).into();
                Err(LockError::UsedByOthers(pid))
            }
        }
    }

    pub fn lock<T: DriverGeneric>(self: &Arc<Self>) -> Result<DeviceGuard<T>, LockError> {
        if !self.is::<T>() {
            return Err(LockError::TypeNotMatch);
        }
        let pid = get_pid();
        loop {
            match self.try_lock(pid, false) {
                Ok(guard) => return Ok(guard),
                Err(LockError::UsedByOthers(_)) => continue,
                Err(e) => return Err(e),
            }
        }
    }
}

pub struct DeviceGuard<T: DriverGeneric> {
    lock: Arc<LockInner>,
    mark: PhantomData<T>,
}

impl<T: DriverGeneric> Drop for DeviceGuard<T> {
    fn drop(&mut self) {
        self.lock.borrowed.store(-1, Ordering::Release);
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

pub struct DeviceWeak {
    lock: Weak<LockInner>,
}

impl DeviceWeak {
    fn new(lock: &Arc<LockInner>) -> Self {
        Self {
            lock: Arc::downgrade(lock),
        }
    }

    pub fn try_lock<T: DriverGeneric>(&self) -> Result<DeviceGuard<T>, LockError> {
        self.lock
            .upgrade()
            .ok_or(LockError::DeviceReleased)?
            .try_lock(get_pid(), true)
    }
    pub fn lock<T: DriverGeneric>(&self) -> Result<DeviceGuard<T>, LockError> {
        self.lock.upgrade().ok_or(LockError::DeviceReleased)?.lock()
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

    pub fn lock(&self) -> Result<DeviceGuard<T>, LockError> {
        self.dev.lock()
    }
    pub fn try_lock(&self) -> Result<DeviceGuard<T>, LockError> {
        self.dev.try_lock()
    }
}

#[derive(thiserror::Error, Debug, Clone, Copy)]
pub enum LockError {
    #[error("used by pid: {0:?}")]
    UsedByOthers(Pid),
    #[error("device type not match")]
    TypeNotMatch,
    #[error("device released")]
    DeviceReleased,
}
