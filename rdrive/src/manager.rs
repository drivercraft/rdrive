use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use rdif_clk::DriverGeneric;

use crate::{
    Descriptor, Device, DeviceId, DeviceOwner, DeviceWeak, GetDeviceError, Platform,
    error::DriverError,
    probe::{EnumSystem, EnumSystemTrait, ProbeError, UnprobedDevice},
    register::{DriverRegisterData, RegisterContainer},
};

pub struct Manager {
    pub registers: RegisterContainer,
    pub(crate) dev_container: DeviceContainer,
    pub(crate) enum_system: EnumSystem,
}

impl Manager {
    pub fn new(platform: Platform) -> Result<Self, DriverError> {
        Ok(Self {
            enum_system: EnumSystem::new(platform)?,
            registers: RegisterContainer::default(),
            dev_container: DeviceContainer::default(),
        })
    }

    pub fn to_unprobed(
        &mut self,
        register: &DriverRegisterData,
    ) -> Result<Option<UnprobedDevice>, ProbeError> {
        let unprobed = self.enum_system.to_unprobed(register)?;
        if unprobed.is_some() {
            self.registers.set_probed(register.id);
        }
        Ok(unprobed)
    }

    pub fn unregistered(&mut self) -> Result<Vec<DriverRegisterData>, ProbeError> {
        let mut out = self.registers.unregistered();
        out.sort_by(|a, b| a.register.priority.cmp(&b.register.priority));
        Ok(out)
    }
}

#[derive(Default)]
pub(crate) struct DeviceContainer {
    devices: BTreeMap<DeviceId, DeviceOwner>,
}

impl DeviceContainer {
    pub fn insert<T: DriverGeneric + 'static>(&mut self, id: DeviceId, device: T) {
        self.devices
            .insert(id, DeviceOwner::new(Descriptor::default(), device));
    }

    pub fn get_typed<T: DriverGeneric>(&self, id: DeviceId) -> Result<Device<T>, GetDeviceError> {
        let dev = self.devices.get(&id).ok_or(GetDeviceError::NotFound)?;

        dev.weak_typed()
    }

    pub fn get(&self, id: DeviceId) -> Option<DeviceWeak> {
        let dev = self.devices.get(&id)?;
        Some(dev.weak())
    }

    pub fn get_one<T: DriverGeneric>(&self) -> Option<Device<T>> {
        for dev in self.devices.values() {
            if let Ok(val) = dev.weak_typed::<T>() {
                return Some(val);
            }
        }
        None
    }

    pub fn devices<T: DriverGeneric>(&self) -> Vec<Device<T>> {
        let mut result = Vec::new();
        for dev in self.devices.values() {
            if let Ok(val) = dev.weak_typed::<T>() {
                result.push(val);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::device::{self, Empty};

    use super::*;

    struct DeviceTest {
        opened: bool,
    }

    impl DriverGeneric for DeviceTest {
        fn open(&mut self) -> Result<(), rdif_base::KError> {
            self.opened = true;
            Ok(())
        }

        fn close(&mut self) -> Result<(), rdif_base::KError> {
            if !self.opened {
                panic!("Device not opened before closing");
            }
            self.opened = false;
            Ok(())
        }
    }

    #[test]
    fn test_device_container() {
        let mut container = DeviceContainer::default();
        let id = DeviceId::new();
        container.insert(id, device::Empty);
        let weak = container.get_typed::<device::Empty>(id).unwrap();

        {
            let mut device = weak.lock().unwrap();

            assert!(device.open().is_ok());
            assert!(device.close().is_ok());
        }

        {
            let mut device = weak.lock().unwrap();

            assert!(device.open().is_ok());
            assert!(device.close().is_ok());
        }
    }
    #[test]
    fn test_get_one() {
        let mut container = DeviceContainer::default();
        let id1 = DeviceId::new();
        let id2 = DeviceId::new();
        container.insert(id1, Empty);
        container.insert(id2, DeviceTest { opened: false });

        let weak = container.get_one::<Empty>().unwrap();
        {
            let mut device = weak.lock().unwrap();
            assert!(device.open().is_ok());
            assert!(device.close().is_ok());
        }
    }

    #[test]
    fn test_devices() {
        let mut container = DeviceContainer::default();
        let id1 = DeviceId::new();
        let id2 = DeviceId::new();
        container.insert(id1, Empty);
        container.insert(id2, Empty);
        container.insert(DeviceId::new(), DeviceTest { opened: false });
        let devices = container.devices::<Empty>();
        assert_eq!(devices.len(), 2);
    }

    #[test]
    fn test_not_found() {
        let container = DeviceContainer::default();
        let dev = container.get_one::<device::Intc>();
        assert!(dev.is_none(), "Expected no devices found");

        if let Some(dev) = dev {
            let weak = dev.lock().unwrap();
            let f = weak.parse_dtb_fn();
            assert!(f.is_none(), "Expected no parse function for empty device");
        }
    }
}
