use alloc::{collections::btree_map::BTreeMap, vec::Vec};

use crate::{
    DeviceId, DeviceKind, DriverInfoKind, DriverRegister,
    probe::{EnumSystem, ProbeError, ProbedDevice, UnprobedDevice},
    register::{DriverRegisterData, ProbeLevel, RegisterContainer},
};

#[derive(Default)]
pub struct Manager {
    pub registers: RegisterContainer,
    pub dev_map: BTreeMap<DeviceId, DeviceKind>,
    pub enum_system: EnumSystem,
}

impl Manager {
    pub fn new(driver_info_kind: DriverInfoKind) -> Self {
        Self {
            enum_system: driver_info_kind.into(),
            ..Default::default()
        }
    }

    pub fn probe(&mut self, register: &DriverRegisterData) -> Result<(), ProbeError> {
        let dev = self.enum_system.probe(register)?;
        if let Some(dev) = dev {
            self.add_probed(dev);
        }
        Ok(())
    }

    pub fn unregistered(&self) -> Result<Vec<DriverRegisterData>, ProbeError> {
        let mut out = self.registers.unregistered();
        out.sort_by(|a, b| a.register.priority.cmp(&b.register.priority));
        Ok(out)
    }

    fn add_probed(&mut self, probed: ProbedDevice) {
        self.registers.set_probed(probed.register_id);
        self.dev_map.insert(probed.descriptor.device_id, probed.dev);
    }
}
