use alloc::{collections::btree_map::BTreeMap, vec::Vec};

use crate::{
    DeviceId, DeviceKind, DriverInfoKind, DriverRegister,
    probe::{EnumSystem, ProbeError},
    register::{ProbeLevel, RegisterContainer},
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

    pub fn probe_pre_kernel(&mut self) -> Result<(), ProbeError> {
        let ls = self
            .registers
            .unregistered()
            .into_iter()
            .filter(|(_, e)| matches!(e.level, ProbeLevel::PreKernel))
            .collect::<Vec<_>>();

        self.probe_with(&ls)
    }

    pub fn probe(&mut self) -> Result<(), ProbeError> {
        let ls = self.registers.unregistered();

        self.probe_with(&ls)
    }

    fn probe_with(&mut self, registers: &[(usize, DriverRegister)]) -> Result<(), ProbeError> {
        let mut sorted = registers.to_vec();
        sorted.sort_by(|a, b| a.1.priority.cmp(&b.1.priority));

        let probed_list = match &mut self.enum_system {
            EnumSystem::Fdt(probe_data) => probe_data.probe(&sorted)?,
        };

        for probed in probed_list {
            self.registers.set_probed(probed.register_id);
            self.dev_map.insert(probed.descriptor.device_id, probed.dev);
        }

        Ok(())
    }
}
