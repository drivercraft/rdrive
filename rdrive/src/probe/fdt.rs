use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};
use core::{error::Error, ptr::NonNull};
use log::debug;

use fdt_parser::{Fdt, Node, Phandle, Status};
use rdif_base::IrqConfig;
pub use rdif_intc::FdtParseConfigFn;

use crate::{
    Descriptor, Device, DeviceId, DeviceKind, DriverRegister, ProbedDevice,
    error::DriverError,
    register::{FdtInfo, OnProbeKindFdt, ProbeKind},
};

pub struct ProbeData {
    phandle_2_device_id: BTreeMap<Phandle, DeviceId>,
    phandle_2_irq_parse: BTreeMap<Phandle, FdtParseConfigFn>,
    fdt_addr: NonNull<u8>,
}

unsafe impl Send for ProbeData {}

impl ProbeData {
    pub fn new(fdt_addr: NonNull<u8>) -> Self {
        Self {
            phandle_2_device_id: Default::default(),
            phandle_2_irq_parse: Default::default(),
            fdt_addr,
        }
    }

    pub fn phandle_2_device_id(&self, phandle: Phandle) -> Option<DeviceId> {
        self.phandle_2_device_id.get(&phandle).copied()
    }

    pub fn parse_irq(
        &self,
        parent: Phandle,
        irq_cell: &[u32],
    ) -> Result<IrqConfig, Box<dyn Error>> {
        let f = self.phandle_2_irq_parse[&parent];
        f(irq_cell)
    }

    pub fn probe(
        &mut self,
        registers: &[(usize, DriverRegister)],
    ) -> Result<Vec<ProbedDevice>, DriverError> {
        let fdt = Fdt::from_ptr(self.fdt_addr)?;
        let registers = self.get_all_fdt_registers(registers, &fdt)?;

        self.probe_with(&registers)
    }

    fn probe_with(
        &mut self,
        registers: &[ProbeFdtInfo<'_>],
    ) -> Result<Vec<ProbedDevice>, DriverError> {
        let mut out = Vec::new();

        for register in registers {
            let mut descriptor = register.descriptor.clone();
            debug!("Probe {}", descriptor.name);
            let kind = match register.on_probe {
                OnProbeKindFdt::Intc(on_probe) => {
                    let info = on_probe(register.node.clone())?;
                    let phandle = register
                        .node
                        .phandle()
                        .ok_or(DriverError::Fdt("intc no phandle".into()))?;
                    self.phandle_2_irq_parse
                        .insert(phandle, info.fdt_parse_config_fn);

                    let device_id = descriptor.device_id;

                    self.phandle_2_device_id.insert(phandle, device_id);
                    DeviceKind::Intc(Device::new(descriptor, info.hardware))
                }
                OnProbeKindFdt::Timer(on_probe) => {
                    let parent = register
                        .node
                        .interrupt_parent()
                        .ok_or(DriverError::Fdt("timer no interrupt parent".into()))?
                        .node
                        .phandle()
                        .ok_or(DriverError::Fdt("interrupt controller no phandle".into()))?;

                    let irq_parse = *self.phandle_2_irq_parse.get(&parent).unwrap();
                    let info = FdtInfo {
                        node: register.node.clone(),
                        irq_parse,
                    };

                    descriptor.irq_parent = self.phandle_2_device_id.get(&parent).cloned();

                    debug!("irq parent: {:?}", descriptor.irq_parent);

                    let hardware = on_probe(info)?;

                    DeviceKind::Timer(Device::new(descriptor, hardware))
                }
            };

            out.push(ProbedDevice {
                register_id: register.register_index,
                kind,
            });
        }

        Ok(out)
    }

    pub fn get_all_fdt_registers<'a>(
        &self,
        registers: &[(usize, DriverRegister)],
        fdt: &'a Fdt<'_>,
    ) -> Result<Vec<ProbeFdtInfo<'a>>, DriverError> {
        let mut vec = Vec::new();
        for node in fdt.all_nodes() {
            if matches!(node.status(), Some(Status::Disabled)) {
                continue;
            }

            let node_compatibles = node.compatibles().collect::<Vec<_>>();

            for (i, register) in registers {
                for probe in register.probe_kinds {
                    match probe {
                        ProbeKind::Fdt {
                            compatibles,
                            on_probe,
                        } => {
                            for campatible in &node_compatibles {
                                if compatibles.contains(campatible) {
                                    vec.push(ProbeFdtInfo {
                                        node: node.clone(),
                                        on_probe: on_probe.clone(),
                                        descriptor: Descriptor {
                                            name: register.name,
                                            device_id: DeviceId::new(),
                                            ..Default::default()
                                        },
                                        register_index: *i,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(vec)
    }
}

pub struct ProbeFdtInfo<'a> {
    pub node: Node<'a>,
    pub on_probe: OnProbeKindFdt,
    pub descriptor: Descriptor,
    register_index: usize,
}
