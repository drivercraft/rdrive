#![no_std]

extern crate alloc;

pub use pci_types::PciAddress;
use rdif_base::def_driver;
pub use rdif_base::{DriverGeneric, KError};

pub trait Interface: DriverGeneric {
    /// Performs a PCI read at `address` with `offset`.
    fn read(&mut self, address: PciAddress, offset: u16) -> u32;

    /// Performs a PCI write at `address` with `offset`.
    fn write(&mut self, address: PciAddress, offset: u16, value: u32);
}

def_driver!(Pcie, Interface);
