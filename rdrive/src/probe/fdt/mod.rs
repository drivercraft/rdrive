use alloc::{boxed::Box, collections::BTreeMap, string::ToString, vec::Vec};
use core::{error::Error, ptr::NonNull};
use log::debug;
use rdif_intc::Capability;

pub use fdt_parser::*;
pub use rdif_intc::FuncFdtParseConfig;

use crate::{
    Descriptor, DeviceId, HardwareKind, get_dev,
    register::{DriverRegisterData, ProbeKind, RegisterId},
};

use super::{ProbeError, ProbedDevice, UnprobedDevice};

#[derive(Clone)]
pub struct FdtInfo<'a> {
    pub node: Node<'a>,
    phandle_2_device_id: BTreeMap<Phandle, DeviceId>,
}

impl FdtInfo<'_> {
    pub fn phandle_to_device_id(&self, phandle: Phandle) -> Option<DeviceId> {
        self.phandle_2_device_id.get(&phandle).copied()
    }

    pub fn find_clk_by_name(&self, name: &str) -> Option<ClockRef> {
        self.node.clocks().find(|clock| clock.name == Some(name))
    }
}

pub type FnOnProbe =
    fn(fdt: FdtInfo<'_>, desc: &Descriptor) -> Result<HardwareKind, Box<dyn Error>>;

pub struct ProbeFunc {
    phandle_2_device_id: BTreeMap<Phandle, DeviceId>,
    fdt_addr: NonNull<u8>,
}

unsafe impl Send for ProbeFunc {}

impl ProbeFunc {
    pub fn new(fdt_addr: NonNull<u8>) -> Self {
        Self {
            phandle_2_device_id: Default::default(),
            fdt_addr,
        }
    }

    pub fn init(&mut self) -> Result<(), ProbeError> {
        let fdt = Fdt::from_ptr(self.fdt_addr)?;
        for node in fdt.all_nodes() {
            if let Some(phandle) = node.phandle() {
                self.phandle_2_device_id.insert(phandle, DeviceId::new());
            }
        }

        Ok(())
    }

    fn new_device_id(&self, phandle: Option<Phandle>) -> DeviceId {
        if let Some(phandle) = phandle {
            self.phandle_2_device_id[&phandle]
        } else {
            DeviceId::new()
        }
    }

    pub fn to_unprobed(
        &mut self,
        register: &DriverRegisterData,
    ) -> Result<Option<UnprobedDevice>, ProbeError> {
        let fdt: Fdt<'static> = Fdt::from_ptr(self.fdt_addr)?;
        let register = match self.get_fdt_register(register, &fdt) {
            Some(v) => v,
            None => return Ok(None),
        };

        let id = self.new_device_id(register.node.phandle());

        let irq_parent = register
            .node
            .interrupt_parent()
            .filter(|p| p.node.phandle() != register.node.phandle())
            .and_then(|n| n.node.phandle())
            .and_then(|p| self.phandle_2_device_id.get(&p).copied());

        let phandle_map = self.phandle_2_device_id.clone();

        let probe_fn = move || {
            debug!("Probe [{}]->[{}]", register.node.name, register.name);
            let mut irqs = Vec::new();

            if let Some(parent) = irq_parent {
                if let Some(raws) = register.node.interrupts() {
                    let intc = get_dev!(parent, Intc).ok_or(ProbeError::IrqNotInit {
                        name: register.name.to_string(),
                    })?;
                    let parse_fn = {
                        let mut found = None;
                        let g = intc.spin_try_borrow_by(0.into())?;
                        #[allow(irrefutable_let_patterns)]
                        for cap in g.capabilities() {
                            if let Capability::FdtParseConfig(f) = cap {
                                found = Some(f);
                            }
                        }
                        found
                    };

                    let parse_fn = parse_fn.ok_or(ProbeError::Fdt(
                        "irq parent does not have irq parse fn".to_string(),
                    ))?;

                    for raw in raws {
                        if let Ok(irq) = parse_fn(&raw.collect::<Vec<_>>()) {
                            irqs.push(irq);
                        }
                    }
                }
            }

            let descriptor = Descriptor {
                name: register.name,
                device_id: id,
                irq_parent,
                irqs: irqs.clone(),
            };

            let dev = (register.on_probe)(
                FdtInfo {
                    node: register.node.clone(),
                    phandle_2_device_id: phandle_map,
                },
                &descriptor,
            )
            .map_err(ProbeError::OnProbe)?;

            let dev = dev.to_device(descriptor.clone());
            Ok(ProbedDevice {
                register_id: register.register_id,
                descriptor,
                dev,
            })
        };

        Ok(Some(Box::new(probe_fn)))
    }

    fn get_fdt_register(
        &self,
        register: &DriverRegisterData,
        fdt: &Fdt<'static>,
    ) -> Option<ProbeFdtInfo> {
        for node in fdt.all_nodes() {
            if matches!(node.status(), Some(Status::Disabled)) {
                continue;
            }

            let node_compatibles = node.compatibles().collect::<Vec<_>>();

            for probe in register.register.probe_kinds {
                match probe {
                    &ProbeKind::Fdt {
                        compatibles,
                        on_probe,
                    } => {
                        for campatible in &node_compatibles {
                            if compatibles.contains(campatible) {
                                return Some(ProbeFdtInfo {
                                    register_id: register.id,
                                    name: register.register.name,
                                    node: node.clone(),
                                    on_probe,
                                });
                            }
                        }
                    }
                }
            }
        }
        None
    }
}

struct ProbeFdtInfo {
    register_id: RegisterId,
    name: &'static str,
    node: Node<'static>,
    on_probe: FnOnProbe,
}
