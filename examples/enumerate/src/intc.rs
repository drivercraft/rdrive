use log::debug;
use rdif_intc::*;
use rdrive::{
    PlatformDevice,
    probe::OnProbeError,
    register::{DriverRegister, FdtInfo, ProbeKind, ProbeLevel, ProbePriority},
};

pub struct IrqTest {}

pub fn register() -> DriverRegister {
    DriverRegister {
        name: "IrqTest",
        probe_kinds: &[ProbeKind::Fdt {
            compatibles: &["arm,cortex-a15-gic"],
            on_probe: probe_intc,
        }],
        level: ProbeLevel::PreKernel,
        priority: ProbePriority::INTC,
    }
}

impl rdrive::DriverGeneric for IrqTest {
    fn open(&mut self) -> Result<(), KError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), KError> {
        Ok(())
    }
}

impl Interface for IrqTest {
    fn parse_dtb_fn(&self) -> Option<rdif_intc::FuncFdtParseConfig> {
        Some(fdt_parse)
    }
}

fn fdt_parse(_prop_interrupts_one_cell: &[u32]) -> Result<IrqConfig, String> {
    Ok(IrqConfig {
        irq: 0.into(),
        trigger: Trigger::EdgeBoth,
        is_private: false,
    })
}

fn probe_intc(fdt: FdtInfo<'_>, plat_dev: PlatformDevice) -> Result<(), OnProbeError> {
    debug!(
        "on_probe: {}, parent intc {:?}",
        fdt.node.name(),
        plat_dev.descriptor.irq_parent,
    );
    plat_dev.register(Intc::new(IrqTest {}));

    Ok(())
}
