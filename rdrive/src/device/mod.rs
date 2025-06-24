use core::ops::{Deref, DerefMut};

use alloc::boxed::Box;
use rdif_base::DriverGeneric;
pub use rdif_base::lock::{LockError, PId};

pub struct Empty;

impl DriverGeneric for Empty {
    fn open(&mut self) -> Result<(), rdif_base::KError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), rdif_base::KError> {
        Ok(())
    }
}

pub struct Intc(Box<dyn rdif_intc::Interface>);

impl Intc {
    pub fn new<T: rdif_intc::Interface>(driver: T) -> Self {
        Self(Box::new(driver))
    }
}

impl DriverGeneric for Intc {
    fn open(&mut self) -> Result<(), rdif_base::KError> {
        self.0.open()
    }

    fn close(&mut self) -> Result<(), rdif_base::KError> {
        self.0.close()
    }
}

impl Deref for Intc {
    type Target = dyn rdif_intc::Interface;

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl DerefMut for Intc {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}
