use alloc::{collections::btree_map::BTreeMap, vec::Vec};

use crate::{
    DeviceId, DeviceKind, DriverInfoKind,
    probe::{EnumSystem, ProbeError, ProbedDevice, UnprobedDevice},
    register::{DriverRegisterData, RegisterContainer},
};

#[derive(Default)]
pub struct Manager {
    pub registers: RegisterContainer,
    pub dev_map: BTreeMap<DeviceId, DeviceKind>,
    pub enum_system: EnumSystem,
    initialized: bool,
}

impl Manager {
    pub fn new(driver_info_kind: DriverInfoKind) -> Self {
        Self {
            enum_system: driver_info_kind.into(),
            ..Default::default()
        }
    }

    pub fn to_unprobed(
        &mut self,
        register: &DriverRegisterData,
    ) -> Result<Option<UnprobedDevice>, ProbeError> {
        self.enum_system.to_unprobed(register)
    }

    pub fn unregistered(&mut self) -> Result<Vec<DriverRegisterData>, ProbeError> {
        if !self.initialized {
            self.enum_system.init()?;
            self.initialized = true;
        }

        let mut out = self.registers.unregistered();
        out.sort_by(|a, b| a.register.priority.cmp(&b.register.priority));
        Ok(out)
    }

    pub fn add_probed(&mut self, probed: ProbedDevice) {
        self.registers.set_probed(probed.register_id);
        self.dev_map.insert(probed.descriptor.device_id, probed.dev);
    }
}
