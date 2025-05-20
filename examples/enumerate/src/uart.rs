use std::error::Error;

use log::debug;
use rdrive::{
    Descriptor, ErrorBase, HardwareKind,
    register::{DriverRegister, Node, ProbeKind, ProbeLevel, ProbePriority},
    systick::*,
};

struct Timer;

pub fn register() -> DriverRegister {
    DriverRegister {
        name: "PL011",
        probe_kinds: &[ProbeKind::Fdt {
            compatibles: &["arm,pl011"],
            on_probe: probe,
        }],
        level: ProbeLevel::PostKernel,
        priority: ProbePriority::DEFAULT,
    }
}

fn probe(_node: Node<'_>, desc: &Descriptor) -> Result<HardwareKind, Box<dyn Error>> {
    debug!("{desc:?}");

    let clk = desc.get_clk_by_name("apb_pclk").unwrap();

    debug!("clk: {clk:?}");

    let _dev = clk.get_dev().unwrap();

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
