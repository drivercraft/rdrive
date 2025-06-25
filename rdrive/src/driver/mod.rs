use rdif_base::DriverGeneric;

use crate::Descriptor;

#[macro_use]
mod _macros;

pub struct Empty;

impl DriverGeneric for Empty {
    fn open(&mut self) -> Result<(), rdif_base::KError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), rdif_base::KError> {
        Ok(())
    }
}

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
    pub fn register<T: DriverGeneric>(self, driver: T) {
        crate::edit(|manager| {
            manager.dev_container.insert(self.descriptor, driver);
        });
    }
}

def_driver_rdif!(Intc);
def_driver_rdif!(Clk);
def_driver_rdif!(Power);
def_driver_rdif!(Systick);
def_driver_rdif!(Serial);
def_driver_rdif!(Block);
