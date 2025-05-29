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
        name: "PL011",
        probe_kinds: &[ProbeKind::Fdt {
            compatibles: &["arm,pl011"],
            on_probe: probe,
        }],
        level: ProbeLevel::PostKernel,
        priority: ProbePriority::DEFAULT,
    }
}

fn probe(fdt: FdtInfo<'_>, desc: &Descriptor) -> Result<HardwareKind, Box<dyn Error>> {
    debug!("{desc:?}");

    let clk = fdt.find_clk_by_name("apb_pclk").unwrap();

    debug!("clk: {clk:?}");

    let id = fdt
        .phandle_to_device_id(clk.node.phandle().unwrap())
        .unwrap();

    let _clk_dev = get_dev!(id, Clk).unwrap();

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
