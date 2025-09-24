use alloc::vec::Vec;
use core::ops::Deref;

use crate::probe::fdt;
pub use crate::probe::fdt::FdtInfo;
pub use fdt_parser::Node;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct ProbePriority(pub usize);

impl ProbePriority {
    pub const CLK: ProbePriority = ProbePriority(6);
    pub const INTC: ProbePriority = ProbePriority(10);
    pub const DEFAULT: ProbePriority = ProbePriority(256);
}

impl From<usize> for ProbePriority {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProbeLevel {
    PreKernel,
    PostKernel,
}

impl ProbeLevel {
    pub const fn new() -> Self {
        Self::PostKernel
    }
}

impl Default for ProbeLevel {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct DriverRegister {
    pub name: &'static str,
    pub level: ProbeLevel,
    pub priority: ProbePriority,
    pub probe_kinds: &'static [ProbeKind],
}

unsafe impl Send for DriverRegister {}
unsafe impl Sync for DriverRegister {}

pub enum ProbeKind {
    Fdt {
        compatibles: &'static [&'static str],
        on_probe: fdt::FnOnProbe,
    },
    Pci {},
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
        if self.len == 0 {
            return &[];
        }
        unsafe {
            core::slice::from_raw_parts(self.data as _, self.len / size_of::<DriverRegister>())
        }
    }
    pub fn empty() -> Self {
        Self {
            data: core::ptr::null(),
            len: 0,
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
}

impl RegisterContainer {
    pub const fn new() -> Self {
        Self {
            registers: Vec::new(),
        }
    }

    pub fn add(&mut self, register: DriverRegister) {
        self.registers.push(register);
    }

    pub fn append(&mut self, register: &[DriverRegister]) {
        for one in register {
            self.add(one.clone());
        }
    }

    pub fn unregistered(&self) -> Vec<DriverRegister> {
        self.registers.to_vec()
    }
}
