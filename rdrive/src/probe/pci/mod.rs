use core::ops::DerefMut;

use ::pcie::*;
use alloc::{collections::btree_set::BTreeSet, vec::Vec};
use spin::{Mutex, Once};

use crate::{
    Descriptor, Device, DeviceGuard, PlatformDevice, ProbeError, get_list,
    probe::OnProbeError,
    register::{DriverRegister, ProbeKind},
};

static PCIE: Once<Mutex<Vec<PcieEnumterator>>> = Once::new();

pub type FnOnProbe = fn(ep: Endpoint, plat_dev: PlatformDevice) -> Result<(), OnProbeError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Id {
    vendor: u16,
    device: u16,
}

fn pcie() -> &'static Mutex<Vec<PcieEnumterator>> {
    PCIE.call_once(|| {
        let ctrl_ls = get_list::<rdif_pcie::Pcie>();
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
    ctrl: Device<rdif_pcie::Pcie>,
    probed: BTreeSet<Id>,
}

impl PcieEnumterator {
    fn probe(
        &mut self,
        registers: &[DriverRegister],
        stop_if_fail: bool,
    ) -> Result<(), ProbeError> {
        let g = self.ctrl.lock().unwrap();
        let mut host = pcie::RootComplex::new(g);

        for ep in host.enumerate(None) {
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

    fn probe_one(&mut self, ep: Endpoint, registers: &[DriverRegister]) -> Result<(), ProbeError> {
        let id = Id {
            vendor: ep.vendor_id(),
            device: ep.device_id(),
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

            let plat_dev = PlatformDevice::new(desc);

            match (pci_probe)(ep, plat_dev) {
                Ok(_) => {}
                Err(OnProbeError::NotMatch) => continue,
                Err(e) => return Err(e.into()),
            }

            self.probed.insert(id);
            break;
        }

        Ok(())
    }
}

impl Controller for DeviceGuard<rdif_pcie::Pcie> {
    fn read(&mut self, address: PciAddress, offset: u16) -> u32 {
        self.deref_mut().read(address, offset)
    }

    fn write(&mut self, address: PciAddress, offset: u16, value: u32) {
        self.deref_mut().write(address, offset, value)
    }
}
