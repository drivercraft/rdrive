use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use core::ops::Deref;

use crate::probe::fdt;
pub use fdt_parser::Node;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct ProbePriority(pub usize);

impl ProbePriority {
    pub const INTC: ProbePriority = ProbePriority(2);
}

impl Default for ProbePriority {
    fn default() -> Self {
        Self(256)
    }
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

impl Default for ProbeLevel {
    fn default() -> Self {
        Self::PostKernel
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

struct RegisterElem {
    pub register: DriverRegister,
    pub probed: bool,
}

#[derive(Default)]
pub struct RegisterContainer {
    id_iter: usize,
    registers: BTreeMap<usize, RegisterElem>,
}

impl RegisterContainer {
    pub const fn new() -> Self {
        Self {
            registers: BTreeMap::new(),
            id_iter: 0,
        }
    }

    pub fn add(&mut self, register: DriverRegister) {
        self.id_iter += 1;
        self.registers.insert(
            self.id_iter,
            RegisterElem {
                register,
                probed: false,
            },
        );
    }

    pub fn append(&mut self, register: &[DriverRegister]) {
        for one in register {
            self.add(one.clone());
        }
    }

    pub fn set_probed(&mut self, register_idx: usize) {
        if let Some(elem) = self.registers.get_mut(&register_idx) {
            elem.probed = true;
        }
    }

    pub fn unregistered(&self) -> Vec<(usize, DriverRegister)> {
        self.registers
            .iter()
            .filter_map(|(id, r)| {
                if r.probed {
                    None
                } else {
                    Some((*id, r.register.clone()))
                }
            })
            .collect()
    }
}
