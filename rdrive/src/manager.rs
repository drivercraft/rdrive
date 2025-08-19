use alloc::{collections::btree_map::BTreeMap, vec::Vec};

use crate::{
    Descriptor, Device, DeviceId, DeviceOwner, GetDeviceError, Platform,
    driver::Class,
    error::DriverError,
    probe::{EnumSystem, EnumSystemTrait, ProbeError, ToProbeFunc},
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
    ) -> Result<Vec<ToProbeFunc>, ProbeError> {
        let unprobed = self.enum_system.to_unprobed(register)?;
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
    pub fn insert<T: Class>(&mut self, descriptor: Descriptor, device: T) {
        self.devices
            .insert(descriptor.device_id, DeviceOwner::new(descriptor, device));
    }

    pub fn get_typed<T: Class>(&self, id: DeviceId) -> Result<Device<T>, GetDeviceError> {
        let dev = self.devices.get(&id).ok_or(GetDeviceError::NotFound)?;

        dev.weak()
    }

    pub fn get_one<T: Class>(&self) -> Option<Device<T>> {
        for dev in self.devices.values() {
            if let Ok(val) = dev.weak::<T>() {
                return Some(val);
            }
        }
        None
    }

    pub fn devices<T: Class>(&self) -> Vec<Device<T>> {
        let mut result = Vec::new();
        for dev in self.devices.values() {
            if let Ok(val) = dev.weak::<T>() {
                result.push(val);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {

    use crate::driver::{Class, DriverGeneric, Empty, Intc, intc};

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

    impl Class for DeviceTest {}

    #[test]
    fn test_device_container() {
        let mut container = DeviceContainer::default();
        let desc = Descriptor::new();
        let id = desc.device_id;
        container.insert(desc, Empty);
        let weak = container.get_typed::<Empty>(id).unwrap();

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
        container.insert(Descriptor::new(), Empty);
        container.insert(Descriptor::new(), DeviceTest { opened: false });

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
        container.insert(Descriptor::new(), Empty);
        container.insert(Descriptor::new(), Empty);
        container.insert(Descriptor::new(), DeviceTest { opened: false });
        let devices = container.devices::<Empty>();
        assert_eq!(devices.len(), 2);
    }

    #[test]
    fn test_not_found() {
        let container = DeviceContainer::default();
        let dev = container.get_one::<Intc>();
        assert!(dev.is_none(), "Expected no devices found");

        if let Some(dev) = dev {
            let weak = dev.lock().unwrap();
            let f = weak.parse_dtb_fn();
            assert!(f.is_none(), "Expected no parse function for empty device");
        }
    }

    struct IrqTest {}

    impl IrqTest {
        fn is_ok(&mut self) -> bool {
            true // Placeholder for actual logic
        }
    }

    impl crate::DriverGeneric for IrqTest {
        fn open(&mut self) -> Result<(), rdif_clk::KError> {
            Ok(())
        }

        fn close(&mut self) -> Result<(), rdif_clk::KError> {
            Ok(())
        }
    }

    impl intc::Interface for IrqTest {}

    #[test]
    fn test_inner_type() {
        let mut container = DeviceContainer::default();
        let desc = Descriptor::new();
        container.insert(desc, Intc::new(IrqTest {}));

        let weak = container.get_one::<Intc>().unwrap();
        {
            let device = weak.lock().unwrap();
            let intc = device.typed_ref::<IrqTest>();
            assert!(intc.is_some(), "Expected to find IrqTest device");
        }
    }

    #[test]
    fn test_device_downcast() {
        let mut container = DeviceContainer::default();
        let desc = Descriptor::new();
        container.insert(desc, Intc::new(IrqTest {}));

        let weak = container.get_one::<Intc>().unwrap();
        let intc_typed = weak.downcast::<IrqTest>().unwrap();
        let mut device = intc_typed.lock().unwrap();
        assert!(device.is_ok(), "Expected device to be ok");
    }

    #[test]
    fn test_locked_device() {
        let mut container = DeviceContainer::default();
        let desc = Descriptor::new();
        let id = desc.device_id;
        container.insert(desc, Empty);

        let weak = container.get_typed::<Empty>(id).unwrap();
        let device = weak.lock().unwrap();
        let r = weak.try_lock();
        assert!(
            r.is_err(),
            "Expected error when trying to lock an already locked device"
        );
        let _ = device;
    }
}
