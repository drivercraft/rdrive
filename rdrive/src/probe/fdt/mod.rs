use alloc::{boxed::Box, collections::BTreeMap, string::ToString, vec::Vec};
use core::ptr::NonNull;
use log::{debug, warn};

pub use fdt_parser::*;
pub use rdif_intc::FuncFdtParseConfig;

use crate::{
    Descriptor, DeviceId, PlatformDevice, driver,
    error::DriverError,
    get,
    probe::OnProbeError,
    register::{DriverRegisterData, ProbeKind},
};

use super::{ProbeError, ToProbeFunc};

#[derive(Clone)]
pub struct FdtInfo<'a> {
    pub node: Node<'a>,
    phandle_2_device_id: BTreeMap<Phandle, DeviceId>,
}

impl<'a> FdtInfo<'a> {
    pub fn phandle_to_device_id(&self, phandle: Phandle) -> Option<DeviceId> {
        self.phandle_2_device_id.get(&phandle).copied()
    }

    pub fn find_clk_by_name(&'a self, name: &str) -> Option<ClockRef<'a>> {
        self.node.clocks().find(|clock| clock.name == Some(name))
    }
}

pub type FnOnProbe = fn(fdt: FdtInfo<'_>, plat_dev: PlatformDevice) -> Result<(), OnProbeError>;

pub struct System {
    phandle_2_device_id: BTreeMap<Phandle, DeviceId>,
    fdt_addr: NonNull<u8>,
}

unsafe impl Send for System {}

impl super::EnumSystemTrait for System {
    fn to_unprobed(
        &mut self,
        register: &DriverRegisterData,
    ) -> Result<Vec<ToProbeFunc>, ProbeError> {
        let fdt: Fdt<'static> = Fdt::from_ptr(self.fdt_addr)?;
        let registers = self.get_fdt_registers(register, &fdt);
        let mut out: Vec<ToProbeFunc> = Vec::new();

        for register in registers {
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

                if let Some(parent) = irq_parent
                    && let Some(raws) = register.node.interrupts()
                {
                    match get::<driver::Intc>(parent) {
                        Ok(intc) => {
                            let parse_fn = { intc.lock().unwrap().parse_dtb_fn() }.ok_or(
                                OnProbeError::Fdt(
                                    "irq parent does not have irq parse fn".to_string(),
                                ),
                            )?;

                            for raw in raws {
                                if let Ok(irq) = parse_fn(&raw.collect::<Vec<_>>()) {
                                    irqs.push(irq);
                                }
                            }
                        }
                        Err(_) => {
                            warn!(
                                "[{}] parent irq driver does not exist, can not parse irq config",
                                register.name
                            );
                        }
                    }
                }

                let descriptor = Descriptor {
                    name: register.name,
                    device_id: id,
                    irq_parent,
                    irqs: irqs.clone(),
                };

                (register.on_probe)(
                    FdtInfo {
                        node: register.node.clone(),
                        phandle_2_device_id: phandle_map,
                    },
                    PlatformDevice::new(descriptor),
                )
            };

            out.push(Box::new(probe_fn));
        }

        Ok(out)
    }
}

impl System {
    pub fn new(fdt_addr: NonNull<u8>) -> Result<Self, DriverError> {
        let fdt = Fdt::from_ptr(fdt_addr)?;
        let mut phandle_2_device_id = BTreeMap::new();
        for node in fdt.all_nodes() {
            if let Some(phandle) = node.phandle() {
                phandle_2_device_id.insert(phandle, DeviceId::new());
            }
        }
        Ok(Self {
            phandle_2_device_id,
            fdt_addr,
        })
    }

    fn new_device_id(&self, phandle: Option<Phandle>) -> DeviceId {
        if let Some(phandle) = phandle {
            self.phandle_2_device_id[&phandle]
        } else {
            DeviceId::new()
        }
    }

    fn get_fdt_registers(
        &self,
        register: &DriverRegisterData,
        fdt: &Fdt<'static>,
    ) -> Vec<ProbeFdtInfo> {
        let mut out = Vec::new();
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
                                out.push(ProbeFdtInfo {
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
        out
    }
}

struct ProbeFdtInfo {
    name: &'static str,
    node: Node<'static>,
    on_probe: FnOnProbe,
}
