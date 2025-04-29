use alloc::{boxed::Box, collections::BTreeMap, format, vec::Vec};
use core::{error::Error, ptr::NonNull};
use log::debug;
use rdif_intc::Capability;

pub use fdt_parser::Node;
use fdt_parser::{Fdt, Phandle, Status};
use rdif_base::IrqConfig;
pub use rdif_intc::FuncFdtParseConfig;

use crate::{Descriptor, DeviceId, DriverRegister, register::ProbeKind};

use super::{HardwareKind, ProbeDevInfo, ProbeError, ProbedDevice};

pub type FnOnProbe =
    fn(node: Node<'_>, dev: ProbeDevInfo) -> Result<Vec<HardwareKind>, Box<dyn Error>>;

pub struct ProbeFunc {
    phandle_2_device_id: BTreeMap<Phandle, DeviceId>,
    phandle_2_irq_parse: BTreeMap<Phandle, FuncFdtParseConfig>,
    fdt_addr: NonNull<u8>,
}

unsafe impl Send for ProbeFunc {}

impl ProbeFunc {
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
        let f = self
            .phandle_2_irq_parse
            .get(&parent)
            .ok_or(format!("{parent} no irq parser"))?;
        f(irq_cell)
    }

    pub fn probe(
        &mut self,
        registers: &[(usize, DriverRegister)],
    ) -> Result<Vec<ProbedDevice>, ProbeError> {
        let fdt = Fdt::from_ptr(self.fdt_addr)?;
        let registers = self.get_all_fdt_registers(registers, &fdt);

        self.probe_with(&registers)
    }

    fn probe_with(
        &mut self,
        registers: &[ProbeFdtInfo<'_>],
    ) -> Result<Vec<ProbedDevice>, ProbeError> {
        let mut out = Vec::new();

        for register in registers {
            debug!("Probe {}", register.node.name);
            let mut irqs = Vec::new();
            let mut irq_parent = None;

            if let Some(parent) = register
                .node
                .interrupt_parent()
                .and_then(|i| i.node.phandle())
            {
                irq_parent = self.phandle_2_device_id.get(&parent).cloned();
                if let Some(raws) = register.node.interrupts() {
                    for raw in raws {
                        if let Ok(irq) = self.parse_irq(parent, &raw.collect::<Vec<_>>()) {
                            irqs.push(irq);
                        }
                    }
                }
            }

            let dev_info = ProbeDevInfo {
                irqs: irqs.clone(),
                irq_parent,
            };

            let mut dev_list = (register.on_probe)(register.node.clone(), dev_info)
                .map_err(ProbeError::OnProbe)?;

            while let Some(dev) = dev_list.pop() {
                let mut descriptor = Descriptor {
                    name: register.name,
                    device_id: DeviceId::new(),
                    irq_parent,
                    irqs: irqs.clone(),
                };

                if let HardwareKind::Intc(intc) = &dev {
                    descriptor.irq_parent = None;
                    let phandle = register
                        .node
                        .phandle()
                        .ok_or(ProbeError::Fdt("intc no phandle".into()))?;

                    let mut parser = None;

                    for cap in intc.capabilities() {
                        match cap {
                            Capability::FdtParseConfig(f) => parser = Some(f),
                        }
                    }

                    let parser = parser.ok_or(ProbeError::Fdt("intc no irq parser".into()))?;

                    self.phandle_2_irq_parse.insert(phandle, parser);

                    self.phandle_2_device_id
                        .insert(phandle, descriptor.device_id);
                }

                out.push(ProbedDevice {
                    register_id: register.register_index,
                    descriptor,
                    dev,
                });
            }
        }

        Ok(out)
    }

    pub fn get_all_fdt_registers<'a>(
        &self,
        registers: &[(usize, DriverRegister)],
        fdt: &'a Fdt<'_>,
    ) -> Vec<ProbeFdtInfo<'a>> {
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
                                        name: register.name,
                                        node: node.clone(),
                                        on_probe: *on_probe,
                                        register_index: *i,
                                    });
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        vec
    }
}

pub struct ProbeFdtInfo<'a> {
    name: &'static str,
    pub node: Node<'a>,
    pub on_probe: FnOnProbe,
    register_index: usize,
}
