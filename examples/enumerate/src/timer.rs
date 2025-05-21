use std::error::Error;

use log::debug;
use rdrive::{
    get_dev, register::{DriverRegister, FdtInfo, Node, ProbeKind, ProbeLevel, ProbePriority}, systick::*, Descriptor, ErrorBase, HardwareKind
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
    let parent = desc.irq_parent.unwrap();

    let intc = get_dev!(parent, Intc).unwrap();

    debug!("intc : {}", intc.descriptor.name);

    Ok(HardwareKind::Systick(Box::new(Timer {})))
}

impl DriverGeneric for Timer {
    fn open(&mut self) -> Result<(), ErrorBase> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), ErrorBase> {
        Ok(())
    }
}

impl Interface for Timer {
    fn get_current_cpu(&mut self) -> Box<dyn InterfaceCPU> {
        todo!()
    }
}
