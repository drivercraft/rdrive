use std::error::Error;

use log::debug;
use rdrive::{
    Descriptor, HardwareKind, get_dev,
    register::{DriverRegister, Node, ProbeKind, ProbeLevel, ProbePriority},
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

fn probe(_node: Node<'_>, desc: &Descriptor) -> Result<HardwareKind, Box<dyn Error>> {
    let parent = desc.irq_parent.unwrap();

    let intc = get_dev!(parent, Intc).unwrap();

    debug!("intc : {}", intc.descriptor.name);

    Ok(HardwareKind::Systick(Box::new(Timer {})))
}

impl DriverGeneric for Timer {
    fn open(&mut self) -> DriverResult {
        Ok(())
    }

    fn close(&mut self) -> DriverResult {
        Ok(())
    }
}

impl Interface for Timer {
    fn get_current_cpu(&mut self) -> Box<dyn InterfaceCPU> {
        todo!()
    }
}
