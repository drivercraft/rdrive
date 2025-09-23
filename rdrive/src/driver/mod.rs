pub use rdif_base::DriverGeneric;

use crate::Descriptor;

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

