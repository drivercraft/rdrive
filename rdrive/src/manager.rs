use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use rdif_base::DriverGeneric;

use crate::{
    Descriptor, Device, DeviceId, DeviceOwner, DeviceWeak, GetDeviceError, Platform,
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
    ) -> Result<Option<ToProbeFunc>, ProbeError> {
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
    pub fn insert<T: DriverGeneric + 'static>(&mut self, descriptor: Descriptor, device: T) {
        self.devices
            .insert(descriptor.device_id, DeviceOwner::new(descriptor, device));
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
    use crate::driver::{self, Empty};

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
        let desc = Descriptor::new();
        let id = desc.device_id;
        container.insert(desc, driver::Empty);
        let weak = container.get_typed::<driver::Empty>(id).unwrap();

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
        let dev = container.get_one::<driver::Intc>();
        assert!(dev.is_none(), "Expected no devices found");

        if let Some(dev) = dev {
            let weak = dev.lock().unwrap();
            let f = weak.parse_dtb_fn();
            assert!(f.is_none(), "Expected no parse function for empty device");
        }
    }

    struct IrqTest {}

    impl crate::DriverGeneric for IrqTest {
        fn open(&mut self) -> Result<(), rdif_clk::KError> {
            Ok(())
        }

        fn close(&mut self) -> Result<(), rdif_clk::KError> {
            Ok(())
        }
    }

    impl driver::intc::Interface for IrqTest {
        fn irq_enable(&mut self, _irq: rdif_intc::IrqId) -> Result<(), rdif_intc::IntcError> {
            todo!()
        }

        fn irq_disable(&mut self, _irq: rdif_intc::IrqId) -> Result<(), rdif_intc::IntcError> {
            todo!()
        }

        fn set_priority(
            &mut self,
            _irq: rdif_intc::IrqId,
            _priority: usize,
        ) -> Result<(), rdif_intc::IntcError> {
            todo!()
        }

        fn set_trigger(
            &mut self,
            _irq: rdif_intc::IrqId,
            _trigger: rdif_intc::Trigger,
        ) -> Result<(), rdif_intc::IntcError> {
            todo!()
        }

        fn set_target_cpu(
            &mut self,
            _irq: rdif_intc::IrqId,
            _cpu: rdif_base::CpuId,
        ) -> Result<(), rdif_intc::IntcError> {
            todo!()
        }

        fn cpu_local(&self) -> Option<rdif_intc::local::Boxed> {
            todo!()
        }
    }

    #[test]
    fn test_inner_type() {
        let mut container = DeviceContainer::default();
        let desc = Descriptor::new();
        container.insert(desc, driver::Intc::new(IrqTest {}));

        let weak = container.get_one::<driver::Intc>().unwrap();
        {
            let device = weak.lock().unwrap();
            let intc = device.typed_ref::<IrqTest>();
            assert!(intc.is_some(), "Expected to find IrqTest device");
        }
    }
}
