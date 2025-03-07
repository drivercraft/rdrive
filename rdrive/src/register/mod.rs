use alloc::{collections::BTreeSet, vec::Vec};
use core::ops::Deref;

use crate::intc::IrqConfig;
use crate::probe::fdt;
pub use fdt_parser::Node;

#[derive(Clone)]
pub struct DriverRegister {
    pub name: &'static str,
    pub kind: DriverKind,
    pub probe_kinds: &'static [ProbeKind],
}

unsafe impl Send for DriverRegister {}
unsafe impl Sync for DriverRegister {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverKind {
    Intc,
    Timer,
    Power,
    Other,
}

pub enum ProbeKind {
    Fdt {
        compatibles: &'static [&'static str],
        on_probe: fdt::FnOnProbe,
    },
}

pub struct FdtInfo<'a> {
    pub node: Node<'a>,
    pub irqs: Vec<IrqConfig>,
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
