use alloc::sync::{Arc, Weak};
use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicI64, Ordering},
};

use crate::custom_type;

custom_type!(PId, usize, "{:?}");

pub enum LockError {
    UsedByOthers(PId),
    DeviceReleased,
}

pub struct Lock<T> {
    data: Arc<LockInner<T>>,
}

impl<T> Lock<T> {
    pub fn new(data: T) -> Self {
        Lock {
            data: Arc::new(LockInner::new(data)),
        }
    }

    pub fn try_borrow(&self, pid: PId) -> Result<LockGuard<T>, LockError> {
        self.data.try_borrow(pid)
    }

    pub fn weak(&self) -> LockWeak<T> {
        LockWeak {
            data: Arc::downgrade(&self.data),
        }
    }

    /// 强制获取设备
    ///
    /// # Safety
    /// 一般用于中断处理中
    pub unsafe fn force_use(&self) -> *mut T {
        self.data.data.get()
    }
}

impl<T: Sync + Send> Deref for Lock<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data.data.get() }
    }
}

pub struct LockWeak<T> {
    data: Weak<LockInner<T>>,
}

impl<T> LockWeak<T> {
    pub fn upgrade(&self) -> Option<Lock<T>> {
        self.data.upgrade().map(|data| Lock { data })
    }

    pub fn try_borrow(&self, pid: PId) -> Result<LockGuard<T>, LockError> {
        self.upgrade()
            .ok_or(LockError::DeviceReleased)?
            .try_borrow(pid)
    }
}

struct LockInner<T> {
    borrowed: AtomicI64,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for LockInner<T> {}
unsafe impl<T: Send> Sync for LockInner<T> {}

impl<T> LockInner<T> {
    fn new(data: T) -> Self {
        LockInner {
            borrowed: AtomicI64::new(-1),
            data: UnsafeCell::new(data),
        }
    }

    pub fn try_borrow(self: &Arc<Self>, pid: PId) -> Result<LockGuard<T>, LockError> {
        let id = pid.0 as i64;

        match self
            .borrowed
            .compare_exchange(-1, id, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(_) => Ok(LockGuard { data: self.clone() }),
            Err(old) => {
                let pid = PId(old as usize);
                Err(LockError::UsedByOthers(pid))
            }
        }
    }
}

pub struct LockGuard<T> {
    data: Arc<LockInner<T>>,
}

impl<T> Drop for LockGuard<T> {
    fn drop(&mut self) {
        self.data.borrowed.store(-1, Ordering::Release);
    }
}

impl<T> Deref for LockGuard<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data.data.get() }
    }
}

impl<T> DerefMut for LockGuard<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.data.data.get() }
    }
}
