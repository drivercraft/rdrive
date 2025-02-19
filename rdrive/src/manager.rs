use alloc::vec::Vec;

use crate::{
    DeviceKind, DriverInfoKind, DriverRegister,
    probe::ProbeData,
    register::{OnProbeKindFdt, ProbeKind, RegisterContainer},
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
            .filter(|(_, e)| {
                let mut has = false;
                for kind in e.probe_kinds {
                    match kind {
                        ProbeKind::Fdt {
                            compatibles: _,
                            on_probe,
                        } => {
                            if matches!(on_probe, OnProbeKindFdt::Intc(_)) {
                                has = true;
                                break;
                            }
                        }
                    }
                }
                has
            })
            .collect::<Vec<_>>();

        self.probe_with(&ls)
    }

    pub fn probe_timer(&mut self) -> Result<(), DriverError> {
        let ls = self
            .registers
            .unregistered()
            .into_iter()
            .filter(|(_, e)| {
                let mut has = false;
                for kind in e.probe_kinds {
                    match kind {
                        ProbeKind::Fdt {
                            compatibles: _,
                            on_probe,
                        } => {
                            if matches!(on_probe, OnProbeKindFdt::Timer(_)) {
                                has = true;
                                break;
                            }
                        }
                    }
                }
                has
            })
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
            ProbeData::Static => Vec::new(),
        };

        for probed in probed_list {
            match probed.kind {
                DeviceKind::Intc(device) => {
                    self.intc.insert(device);
                }
                DeviceKind::Timer(device) => {
                    self.timer.insert(device);
                }
            }
            self.registers.set_probed(probed.register_id);
        }

        Ok(())
    }
}
