use core::fmt::Debug;

mod card_bridge;
mod endpoint;
mod pci_bridge;
mod unknown;

pub use card_bridge::*;
pub use endpoint::Endpoint;
pub use pci_bridge::*;
use rdif_pcie::ConfigAccess;
pub use unknown::*;

use pci_types::{
    CommandRegister, ConfigRegionAccess, HeaderType, PciAddress, PciHeader, StatusRegister,
};

use crate::chip::PcieController;

#[derive(Debug)]
pub enum PciConfigSpace {
    PciPciBridge(PciPciBridge),
    Endpoint(Endpoint),
    CardBusBridge(CardBusBridge),
    Unknown(Unknown),
}

pub struct PciHeaderBase {
    vid: u16,
    did: u16,
    root: ConfigAccess,
    header: PciHeader,
}

impl PciHeaderBase {
    pub(crate) fn new(root: &mut PcieController, address: PciAddress) -> Option<Self> {
        let root = root.config_access(address);
        let header = PciHeader::new(address);
        let (vid, did) = header.id(&root);
        if vid == 0xffff {
            return None;
        }

        Some(Self {
            vid,
            did,
            root,
            header,
        })
    }

    pub fn header(&self) -> PciHeader {
        PciHeader::new(self.address())
    }

    pub fn address(&self) -> PciAddress {
        self.header.address()
    }

    pub fn header_type(&self) -> HeaderType {
        self.header.header_type(&self.root)
    }

    pub fn has_multiple_functions(&self) -> bool {
        self.header.has_multiple_functions(&self.root)
    }

    pub fn update_command<F>(&mut self, f: F)
    where
        F: FnOnce(CommandRegister) -> CommandRegister,
    {
        self.header.update_command(&self.root, f);
    }

    pub fn status(&self) -> StatusRegister {
        self.header.status(&self.root)
    }

    pub fn command(&self) -> CommandRegister {
        self.header.command(&self.root)
    }

    pub fn revision_and_class(&self) -> RevisionAndClass {
        let (revision_id, base_class, sub_class, interface) =
            self.header.revision_and_class(&self.root);
        RevisionAndClass {
            revision_id,
            base_class,
            sub_class,
            interface,
        }
    }

    pub fn vendor_id(&self) -> u16 {
        self.vid
    }

    pub fn device_id(&self) -> u16 {
        self.did
    }

    pub fn read(&self, offset: u16) -> u32 {
        unsafe { self.root.read(self.address(), offset) }
    }

    pub fn write(&self, offset: u16, value: u32) {
        unsafe { self.root.write(self.address(), offset, value) }
    }
}

#[derive(Debug, Clone)]
pub struct RevisionAndClass {
    pub revision_id: u8,
    pub base_class: u8,
    pub sub_class: u8,
    pub interface: u8,
}

impl Debug for PciHeaderBase {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PciHeaderBase")
            .field("address", &self.address())
            .field("vid", &format_args!("{:#06x}", self.vid))
            .field("did", &format_args!("{:#06x}", self.did))
            .field("command", &self.command())
            .field("status", &self.status())
            .field("has_multiple_functions", &self.has_multiple_functions())
            .field("revision_and_class", &self.revision_and_class())
            .finish()
    }
}
