#![feature(used_with_arg)]

use std::{error::Error, ptr::NonNull};

use log::debug;
use rdrive::{
    IrqConfig, IrqId, KError, PlatformDevice, driver, get_list,
    register::{DriverRegister, FdtInfo, ProbeKind, ProbeLevel, ProbePriority},
};

pub mod clk;
pub mod timer;
pub mod uart;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let fdt = include_bytes!("../../../data/qemu.dtb");

    rdrive::init(rdrive::Platform::Fdt {
        addr: NonNull::new(fdt.as_ptr() as usize as _).unwrap(),
    })
    .unwrap();

    let register = DriverRegister {
        name: "IrqTest",
        probe_kinds: &[ProbeKind::Fdt {
            compatibles: &["arm,cortex-a15-gic"],
            on_probe: probe_intc,
        }],
        level: ProbeLevel::PreKernel,
        priority: ProbePriority::INTC,
    };

    rdrive::register_add(register);
    rdrive::register_add(timer::register());
    rdrive::register_add(clk::register());
    rdrive::register_add(uart::register());

    rdrive::probe_pre_kernel().unwrap();

    let intc_list = get_list::<driver::Intc>();
    for intc in intc_list {
        println!("intc: {:?}", intc.descriptor());

        let g = intc.lock().unwrap();

        let t = g.typed_ref::<IrqTest>();
        assert!(t.is_some(), "Intc should be [IrqTest]");
    }

    rdrive::probe_all(true).unwrap();
}

struct IrqTest {}

impl rdrive::DriverGeneric for IrqTest {
    fn open(&mut self) -> Result<(), KError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), KError> {
        Ok(())
    }
}

impl rdrive::driver::intc::Interface for IrqTest {
    fn irq_enable(&mut self, _irq: IrqId) -> Result<(), rdrive::driver::intc::IntcError> {
        todo!()
    }

    fn irq_disable(&mut self, _irq: IrqId) -> Result<(), rdrive::driver::intc::IntcError> {
        todo!()
    }

    fn set_priority(
        &mut self,
        _irq: IrqId,
        _priority: usize,
    ) -> Result<(), rdrive::driver::intc::IntcError> {
        todo!()
    }

    fn set_trigger(
        &mut self,
        _irq: IrqId,
        _trigger: rdrive::driver::intc::Trigger,
    ) -> Result<(), rdrive::driver::intc::IntcError> {
        todo!()
    }

    fn set_target_cpu(
        &mut self,
        _irq: IrqId,
        _cpu: rdrive::driver::intc::CpuId,
    ) -> Result<(), rdrive::driver::intc::IntcError> {
        todo!()
    }

    fn cpu_local(&self) -> Option<rdrive::driver::intc::local::Boxed> {
        todo!()
    }

    fn parse_dtb_fn(&self) -> Option<rdrive::driver::intc::FuncFdtParseConfig> {
        Some(fdt_parse)
    }
}

fn fdt_parse(_prop_interrupts_one_cell: &[u32]) -> Result<IrqConfig, Box<dyn Error>> {
    Ok(IrqConfig {
        irq: 0.into(),
        trigger: rdrive::driver::intc::Trigger::EdgeBoth,
        is_private: false,
    })
}

fn probe_intc(fdt: FdtInfo<'_>, plat_dev: PlatformDevice) -> Result<(), Box<dyn Error>> {
    debug!(
        "on_probe: {}, parent intc {:?}",
        fdt.node.name(),
        plat_dev.descriptor.irq_parent,
    );
    plat_dev.register(rdrive::driver::Intc::new(IrqTest {}));

    Ok(())
}
