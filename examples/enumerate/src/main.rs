use std::{error::Error, ptr::NonNull};

use log::debug;
use rdrive::{
    Descriptor, ErrorBase, HardwareKind,
    intc::{IrqConfig, IrqId},
    register::{DriverRegister, Node, ProbeKind, ProbeLevel, ProbePriority},
};

pub mod clk;
pub mod timer;
pub mod uart;

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let fdt = include_bytes!("../../../data/qemu.dtb");

    rdrive::init(rdrive::DriverInfoKind::Fdt {
        addr: NonNull::new(fdt.as_ptr() as usize as _).unwrap(),
    });
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

    let intc_list = rdrive::dev_list!(Intc);
    for intc in intc_list {
        println!("intc: {:?}", intc.descriptor);

        let _g = intc.spin_try_borrow_by(0.into());
    }

    rdrive::probe_all(true).unwrap();
}

struct IrqTest {}

impl rdrive::intc::DriverGeneric for IrqTest {
    fn open(&mut self) -> Result<(), ErrorBase> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), ErrorBase> {
        Ok(())
    }
}

impl rdrive::intc::Interface for IrqTest {
    fn irq_enable(&mut self, _irq: IrqId) -> Result<(), rdrive::intc::IntcError> {
        todo!()
    }

    fn irq_disable(&mut self, _irq: IrqId) -> Result<(), rdrive::intc::IntcError> {
        todo!()
    }

    fn set_priority(
        &mut self,
        _irq: IrqId,
        _priority: usize,
    ) -> Result<(), rdrive::intc::IntcError> {
        todo!()
    }

    fn set_trigger(
        &mut self,
        _irq: IrqId,
        _trigger: rdrive::intc::Trigger,
    ) -> Result<(), rdrive::intc::IntcError> {
        todo!()
    }

    fn set_target_cpu(
        &mut self,
        _irq: IrqId,
        _cpu: rdrive::intc::CpuId,
    ) -> Result<(), rdrive::intc::IntcError> {
        todo!()
    }

    fn capabilities(&self) -> Vec<rdrive::intc::Capability> {
        vec![rdrive::intc::Capability::FdtParseConfig(fdt_parse)]
    }

    fn cpu_interface(&self) -> rdrive::intc::BoxCPU {
        todo!()
    }
}

fn fdt_parse(_prop_interrupts_one_cell: &[u32]) -> Result<IrqConfig, Box<dyn Error>> {
    Ok(IrqConfig {
        irq: 0.into(),
        trigger: rdrive::intc::Trigger::EdgeBoth,
    })
}

fn probe_intc(node: Node<'_>, desc: &Descriptor) -> Result<HardwareKind, Box<dyn Error>> {
    debug!(
        "on_probe: {}, parent intc {:?}",
        node.name(),
        desc.irq_parent,
    );
    Ok(HardwareKind::Intc(Box::new(IrqTest {})))
}
