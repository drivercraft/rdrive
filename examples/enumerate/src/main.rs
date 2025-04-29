use std::{error::Error, ptr::NonNull};

use log::debug;
use rdrive::{
    DriverResult,
    intc::{IrqConfig, IrqId},
    probe::{HardwareKind, ProbeDevInfo},
    register::{DriverKind, Node, ProbeKind},
};

fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let fdt = include_bytes!("../../../data/qemu.dtb");

    rdrive::init(rdrive::DriverInfoKind::Fdt {
        addr: NonNull::new(fdt.as_ptr() as usize as _).unwrap(),
    });
    let register = rdrive::DriverRegister {
        name: "IrqText",
        kind: DriverKind::Intc,
        probe_kinds: &[ProbeKind::Fdt {
            compatibles: &["arm,cortex-a15-gic"],
            on_probe: probe_intc,
        }],
    };

    rdrive::register_add(register);
    rdrive::probe_with_kind(DriverKind::Intc).unwrap();
}

struct IrqTest {}

impl rdrive::intc::DriverGeneric for IrqTest {
    fn open(&mut self) -> DriverResult {
        todo!()
    }

    fn close(&mut self) -> DriverResult {
        todo!()
    }
}

impl rdrive::intc::Interface for IrqTest {
    fn irq_enable(&mut self, _irq: IrqId) {
        todo!()
    }

    fn irq_disable(&mut self, _irq: IrqId) {
        todo!()
    }

    fn set_priority(&mut self, _irq: IrqId, _priority: usize) {
        todo!()
    }

    fn set_trigger(&mut self, _irq: IrqId, _trigger: rdrive::intc::Trigger) {
        todo!()
    }

    fn set_target_cpu(&mut self, _irq: IrqId, _cpu: rdrive::intc::CpuId) {
        todo!()
    }

    fn capabilities(&self) -> Vec<rdrive::intc::Capability> {
        vec![rdrive::intc::Capability::FdtParseConfig(parser)]
    }

    fn cpu_interface(&self) -> rdrive::intc::HardwareCPU {
        todo!()
    }
}

fn parser(_prop_interrupts_one_cell: &[u32]) -> Result<IrqConfig, Box<dyn Error>> {
    Ok(IrqConfig {
        irq: 0.into(),
        trigger: rdrive::intc::Trigger::EdgeBoth,
    })
}

fn probe_intc(node: Node<'_>, dev: ProbeDevInfo) -> Result<Vec<HardwareKind>, Box<dyn Error>> {
    debug!(
        "on_probe: {}, parent intc {:?}",
        node.name(),
        dev.irq_parent
    );
    Ok(vec![HardwareKind::Intc(Box::new(IrqTest {}))])
}
