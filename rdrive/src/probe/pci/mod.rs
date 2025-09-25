use core::ptr::NonNull;

use ::pcie::*;
use alloc::{collections::btree_set::BTreeSet, vec::Vec};
pub use rdif_pcie::{DriverGeneric, PciAddress, PciMem32, PciMem64, PcieController};
use spin::{Mutex, Once};

use crate::{
    Descriptor, Device, PlatformDevice, ProbeError, get_list,
    probe::OnProbeError,
    register::{DriverRegister, ProbeKind},
};

static PCIE: Once<Mutex<Vec<PcieEnumterator>>> = Once::new();

pub type FnOnProbe = fn(ep: Endpoint, plat_dev: PlatformDevice) -> Result<(), PciProbeError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Id {
    vendor: u16,
    device: u16,
}

pub struct PciProbeError {
    pub ep: Endpoint,
    pub kind: OnProbeError,
}

pub fn new_driver_generic(mmio_base: NonNull<u8>) -> PcieController {
    PcieController::new(PcieGeneric::new(mmio_base))
}

fn pcie() -> &'static Mutex<Vec<PcieEnumterator>> {
    PCIE.call_once(|| {
        let ctrl_ls = get_list::<PcieController>();
        let mut vec = Vec::new();
        for ctrl in ctrl_ls.into_iter() {
            {
                let mut g = ctrl.lock().unwrap();
                g.open().unwrap();
            }

            vec.push(PcieEnumterator {
                ctrl,
                probed: BTreeSet::new(),
            });
        }
        Mutex::new(vec)
    })
}
pub(crate) fn probe_with(
    registers: &[DriverRegister],
    stop_if_fail: bool,
) -> Result<(), ProbeError> {
    let mut pcie_ls = pcie().lock();
    for ctrl in pcie_ls.iter_mut() {
        ctrl.probe(registers, stop_if_fail)?;
    }
    Ok(())
}

struct PcieEnumterator {
    ctrl: Device<PcieController>,
    probed: BTreeSet<Id>,
}

impl PcieEnumterator {
    fn probe(
        &mut self,
        registers: &[DriverRegister],
        stop_if_fail: bool,
    ) -> Result<(), ProbeError> {
        let mut g = self.ctrl.lock().unwrap();

        for ep in enumerate_by_controller(&mut g, None) {
            debug!("PCIe endpiont: {}", ep);
            match self.probe_one(ep, registers) {
                Ok(_) => {} // Successfully probed, move to the next
                Err(e) => {
                    if stop_if_fail {
                        return Err(e);
                    } else {
                        warn!("Probe failed: {e}");
                    }
                }
            }
        }

        Ok(())
    }

    fn probe_one(
        &mut self,
        mut endpoint: Endpoint,
        registers: &[DriverRegister],
    ) -> Result<(), ProbeError> {
        let id = Id {
            vendor: endpoint.vendor_id(),
            device: endpoint.device_id(),
        };
        if self.probed.contains(&id) {
            return Ok(());
        }
        for register in registers {
            let Some(pci_probe) = register.probe_kinds.iter().find_map(|probe| {
                if let ProbeKind::Pci { on_probe } = probe {
                    Some(on_probe)
                } else {
                    None
                }
            }) else {
                continue;
            };
            let mut desc = Descriptor::new();
            desc.name = register.name;
            desc.irq_parent = self.ctrl.descriptor().irq_parent;

            let plat_dev = PlatformDevice::new(desc);

            match (pci_probe)(endpoint, plat_dev) {
                Ok(_) => {
                    break;
                }
                Err(e) => {
                    endpoint = e.ep;
                    match e.kind {
                        OnProbeError::NotMatch => continue,
                        e => {
                            return Err(ProbeError::from(e));
                        }
                    }
                }
            }
        }

        self.probed.insert(id);
        Ok(())
    }
}
