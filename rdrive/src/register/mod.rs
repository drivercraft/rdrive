use alloc::collections::BTreeSet;
use alloc::{boxed::Box, vec::Vec};
use core::{error::Error, ops::Deref};

use crate::intc::{FdtParseConfigFn, IrqConfig};
pub use fdt_parser::Node;

pub mod intc;
pub mod timer;

#[derive(Clone)]
pub struct DriverRegister {
    pub name: &'static str,
    pub probe_kinds: &'static [ProbeKind],
}

unsafe impl Send for DriverRegister {}
unsafe impl Sync for DriverRegister {}

pub enum ProbeKind {
    Fdt {
        compatibles: &'static [&'static str],
        on_probe: OnProbeKindFdt,
    },
}

pub struct FdtInfo<'a> {
    pub node: Node<'a>,
    pub irq_parse: FdtParseConfigFn,
}

impl FdtInfo<'_> {
    pub fn node_irqs(&self) -> Result<Vec<IrqConfig>, Box<dyn Error>> {
        let irqs = match self.node.interrupts() {
            Some(i) => i,
            None => return Ok(Vec::new()),
        };

        let irqs = irqs.map(|one| one.collect::<Vec<_>>()).collect::<Vec<_>>();

        let mut out = Vec::with_capacity(irqs.len());

        for irq_raw in irqs {
            out.push((self.irq_parse)(&irq_raw)?);
        }

        Ok(out)
    }
}

#[derive(Clone)]
pub enum OnProbeKindFdt {
    Intc(intc::OnProbeFdt),
    Timer(timer::OnProbeFdt),
}

#[repr(C)]
pub struct DriverRegisterSlice {
    data: *const u8,
    len: usize,
}

impl DriverRegisterSlice {
    pub fn from_raw(data: &'static [u8]) -> Self {
        Self {
            data: data.as_ptr(),
            len: data.len(),
        }
    }

    pub fn as_slice(&self) -> &[DriverRegister] {
        unsafe {
            core::slice::from_raw_parts(self.data as _, self.len / size_of::<DriverRegister>())
        }
    }
}

impl Deref for DriverRegisterSlice {
    type Target = [DriverRegister];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

#[derive(Default)]
pub struct RegisterContainer {
    registers: Vec<DriverRegister>,
    probed_index: BTreeSet<usize>,
}

impl RegisterContainer {
    pub const fn new() -> Self {
        Self {
            registers: Vec::new(),
            probed_index: BTreeSet::new(),
        }
    }

    pub fn add(&mut self, register: DriverRegister) {
        self.registers.push(register);
    }

    pub fn append(&mut self, register: &[DriverRegister]) {
        self.registers.extend_from_slice(register);
    }

    pub fn set_probed(&mut self, register_idx: usize) {
        self.probed_index.insert(register_idx);
    }

    pub fn unregistered(&self) -> Vec<(usize, DriverRegister)> {
        self.registers
            .iter()
            .enumerate()
            .filter(|(i, _)| !self.probed_index.contains(i))
            .map(|(i, r)| (i, r.clone()))
            .collect()
    }
}
