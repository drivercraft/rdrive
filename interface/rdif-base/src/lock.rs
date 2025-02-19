use alloc::sync::{Arc, Weak};
use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::custom_type;

custom_type!(PId, usize, "{:?}");

pub enum LockError {
    UsedByOthers(PId),
}

pub struct Lock<T> {
    data: Arc<LockData<T>>,
}

impl<T> Lock<T> {
    pub fn new(data: T) -> Self {
        Lock {
            data: Arc::new(LockData::new(data)),
        }
    }

    pub fn try_borrow(&self, pid: PId) -> Result<LockGuard<T>, LockError> {
        match self
            .data
            .borrowed
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
        {
            Ok(_) => {
                unsafe {
                    (*self.data.used.get()).replace(pid);
                }
                Ok(LockGuard {
                    data: self.data.clone(),
                })
            }
            Err(_) => {
                let pid = unsafe { *self.data.used.get() };
                Err(LockError::UsedByOthers(pid.unwrap()))
            }
        }
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
    data: Weak<LockData<T>>,
}

impl<T> LockWeak<T> {
    pub fn upgrade(&self) -> Option<Lock<T>> {
        self.data.upgrade().map(|data| Lock { data })
    }
}

struct LockData<T> {
    borrowed: AtomicBool,
    used: UnsafeCell<Option<PId>>,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for LockData<T> {}
unsafe impl<T: Send> Sync for LockData<T> {}

impl<T> LockData<T> {
    fn new(data: T) -> Self {
        LockData {
            borrowed: AtomicBool::new(false),
            used: UnsafeCell::new(None),
            data: UnsafeCell::new(data),
        }
    }
}

pub struct LockGuard<T> {
    data: Arc<LockData<T>>,
}

impl<T> Drop for LockGuard<T> {
    fn drop(&mut self) {
        unsafe {
            (*self.data.used.get()).take();
        }
        self.data.borrowed.store(false, Ordering::Release);
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
