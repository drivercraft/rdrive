use core::any::Any;

pub use rdif_base::DriverGeneric;

use crate::Descriptor;

#[macro_use]
mod _macros;

pub mod block;

pub use block::Block;

pub struct Empty;

impl DriverGeneric for Empty {
    fn open(&mut self) -> Result<(), rdif_base::KError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), rdif_base::KError> {
        Ok(())
    }
}

impl Class for Empty {}

pub struct PlatformDevice {
    pub descriptor: Descriptor,
}

impl PlatformDevice {
    pub(crate) fn new(descriptor: Descriptor) -> Self {
        Self { descriptor }
    }

    /// Register a device to the driver manager.
    ///
    /// # Panics
    /// This method will panic if the device with the same ID is already added
    pub fn register<T: Class>(self, driver: T) {
        crate::edit(|manager| {
            manager.dev_container.insert(self.descriptor, driver);
        });
    }
}

pub trait Class: DriverGeneric {
    fn raw_any(&self) -> Option<&dyn Any> {
        None
    }
    fn raw_any_mut(&mut self) -> Option<&mut dyn Any> {
        None
    }
}

def_driver_rdif!(Intc);
def_driver_rdif!(Clk);
def_driver_rdif!(Power);
def_driver_rdif!(Systick);
def_driver_rdif!(Serial);
// def_driver_rdif!(Block);
