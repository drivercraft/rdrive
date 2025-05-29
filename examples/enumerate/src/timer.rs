use std::error::Error;

use log::debug;
use rdrive::{
    Descriptor, HardwareKind, KError, get_dev,
    register::{DriverRegister, FdtInfo, ProbeKind, ProbeLevel, ProbePriority},
    systick::*,
};

struct Timer;

pub fn register() -> DriverRegister {
    DriverRegister {
        name: "TimerTest",
        probe_kinds: &[ProbeKind::Fdt {
            compatibles: &["arm,pl031"],
            on_probe: probe,
        }],
        level: ProbeLevel::PreKernel,
        priority: ProbePriority::DEFAULT,
    }
}

fn probe(_node: FdtInfo<'_>, desc: &Descriptor) -> Result<HardwareKind, Box<dyn Error>> {
    if let Some(parent) = desc.irq_parent {
        if let Some(intc) = get_dev!(parent, Intc) {
            debug!("intc : {}", intc.descriptor.name);
        }
    }

    Ok(HardwareKind::Systick(Box::new(Timer {})))
}

impl DriverGeneric for Timer {
    fn open(&mut self) -> Result<(), KError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), KError> {
        Ok(())
    }
}

impl Interface for Timer {
    fn cpu_local(&mut self) -> local::Boxed {
        todo!()
    }
}
