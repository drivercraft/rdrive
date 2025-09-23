extern crate alloc;

use log::debug;
use rdrive::probe::OnProbeError;
use rdrive::register::{DriverRegister, ProbeKind, ProbeLevel, ProbePriority};

use rdrive::{PlatformDevice, register::FdtInfo};

pub fn register() -> DriverRegister {
    DriverRegister {
        name: "Virtio",
        probe_kinds: &[ProbeKind::Fdt {
            compatibles: &["virtio,mmio"],
            on_probe: probe,
        }],
        level: ProbeLevel::PostKernel,
        priority: ProbePriority::DEFAULT,
    }
}

fn probe(info: FdtInfo<'_>, _dev: PlatformDevice) -> Result<(), OnProbeError> {
    let mut reg = info.node.reg().ok_or(OnProbeError::other(format!(
        "[{}] has no reg",
        info.node.name()
    )))?;

    let base_reg = reg.next().unwrap();
    let mmio_size = base_reg.size.unwrap_or(0x1000);

    debug!(
        "virtio block device MMIO base address: {:#x}, size: {}",
        base_reg.address, mmio_size
    );

    Err(OnProbeError::NotMatch)
}
