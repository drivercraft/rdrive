use alloc::vec::Vec;

use crate::{
    Device, DriverInfoKind, DriverRegister,
    probe::{HardwareKind, ProbeData},
    register::{DriverKind, RegisterContainer},
};

use crate::error::DriverError;

use super::device;

#[derive(Default)]
pub struct Manager {
    pub registers: RegisterContainer,
    pub intc: device::intc::Container,
    pub timer: device::timer::Container,
    pub probe_kind: ProbeData,
}

impl Manager {
    pub fn new(driver_info_kind: DriverInfoKind) -> Self {
        Self {
            probe_kind: driver_info_kind.into(),
            ..Default::default()
        }
    }

    pub fn probe_intc(&mut self) -> Result<(), DriverError> {
        let ls = self
            .registers
            .unregistered()
            .into_iter()
            .filter(|(_, e)| matches!(e.kind, DriverKind::Intc))
            .collect::<Vec<_>>();

        self.probe_with(&ls)
    }

    pub fn probe_timer(&mut self) -> Result<(), DriverError> {
        let ls = self
            .registers
            .unregistered()
            .into_iter()
            .filter(|(_, e)| matches!(e.kind, DriverKind::Timer))
            .collect::<Vec<_>>();
        self.probe_with(&ls)
    }

    pub fn probe(&mut self) -> Result<(), DriverError> {
        let ls = self.registers.unregistered();

        self.probe_with(&ls)
    }

    fn probe_with(&mut self, registers: &[(usize, DriverRegister)]) -> Result<(), DriverError> {
        let probed_list = match &mut self.probe_kind {
            ProbeData::Fdt(probe_data) => probe_data.probe(registers)?,
        };

        for probed in probed_list {
            self.registers.set_probed(probed.register_id);
            match probed.dev {
                HardwareKind::Intc(interface) => {
                    self.intc.insert(Device::new(probed.descriptor, interface));
                }
                HardwareKind::Timer(interface) => {
                    self.timer.insert(Device::new(probed.descriptor, interface));
                }
            }
        }

        Ok(())
    }
}
