use std::error::Error;

use log::debug;
use rdrive::{
    Descriptor, ErrorBase, HardwareKind,
    clk::*,
    register::{DriverRegister, FdtInfo, ProbeKind, ProbeLevel, ProbePriority},
};

struct Clock {
    rate: u64,
}

pub fn register() -> DriverRegister {
    DriverRegister {
        name: "APB CLK",
        probe_kinds: &[ProbeKind::Fdt {
            compatibles: &["fixed-clock"],
            on_probe: probe,
        }],
        level: ProbeLevel::PreKernel,
        priority: ProbePriority::CLK,
    }
}

fn probe(_node: FdtInfo<'_>, _desc: &Descriptor) -> Result<HardwareKind, Box<dyn Error>> {
    Ok(HardwareKind::Clk(Box::new(Clock { rate: 0 })))
}

impl DriverGeneric for Clock {
    fn open(&mut self) -> Result<(), ErrorBase> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), ErrorBase> {
        Ok(())
    }
}

impl Interface for Clock {
    fn perper_enable(&mut self) {
        debug!("enable");
    }

    fn get_rate(&self, _id: ClockId) -> Result<u64, ErrorBase> {
        Ok(self.rate)
    }

    fn set_rate(&mut self, _id: ClockId, rate: u64) -> Result<(), ErrorBase> {
        self.rate = rate;
        Ok(())
    }
}
