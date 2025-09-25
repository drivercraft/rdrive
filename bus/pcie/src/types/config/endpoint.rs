use core::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut, Range},
};

use alloc::vec::Vec;
use pci_types::{
    capability::PciCapability, device_type::DeviceType, Bar, BarWriteError, CommandRegister,
    EndpointHeader,
};
use rdif_pcie::{ConfigAccess, SimpleBarAllocator};

pub struct Endpoint {
    base: super::PciHeaderBase,
    header: EndpointHeader,
}

impl Endpoint {
    pub(crate) fn new(
        base: super::PciHeaderBase,
        bar_allocator: Option<&mut SimpleBarAllocator>,
    ) -> Self {
        let header = EndpointHeader::from_header(base.header(), &base.root)
            .expect("EndpointHeader::from_header failed");
        let mut s = Self { base, header };
        if let Some(alloc) = bar_allocator {
            s.realloc_bar(alloc).unwrap();
        }
        s
    }

    pub fn device_type(&self) -> DeviceType {
        let class_info = self.base.revision_and_class();
        DeviceType::from((class_info.base_class, class_info.sub_class))
    }

    pub fn bar(&self, slot: u8) -> Option<Bar> {
        let bars = self.bars();
        assert!(slot < 6, "BAR index out of range");
        bars[slot as usize]
    }

    pub fn bar_mmio(&self, slot: u8) -> Option<Range<usize>> {
        let bar = self.bar(slot)?;
        match bar {
            Bar::Memory32 { address, size, .. } => Some(address as _..(address + size) as _),
            Bar::Memory64 { address, size, .. } => Some(address as _..(address + size) as _),
            Bar::Io { .. } => None,
        }
    }

    fn _bar(&self, slot: u8) -> Option<Bar> {
        assert!(slot < 6, "BAR index out of range");
        self.header.bar(slot, self.access())
    }

    pub fn set_bar(&mut self, slot: u8, value: usize) -> Result<(), BarWriteError> {
        assert!(slot < 6, "BAR index out of range");
        unsafe { self.header.write_bar(slot, &self.base.root, value) }
    }

    pub fn bars(&self) -> [Option<Bar>; 6] {
        let mut bars = [None; 6];
        let mut i = 0;
        while i < 6 {
            bars[i] = self._bar(i as u8);
            if let Some(Bar::Memory64 { .. }) = bars[i] {
                i += 1; // Skip the next BAR since it's part of this 64-bit BAR
            }
            i += 1;
        }
        bars
    }

    pub fn capabilities_pointer(&self) -> u16 {
        self.header.capability_pointer(self.access())
    }

    pub fn capabilities(&self) -> Vec<PciCapability> {
        self.header.capabilities(self.access()).collect()
    }

    pub fn interrupt_pin(&self) -> u8 {
        self.header.interrupt(self.access()).0
    }

    pub fn interrupt_line(&self) -> u8 {
        self.header.interrupt(self.access()).1
    }

    pub fn subsystem_id(&self) -> u16 {
        self.header.subsystem(self.access()).0
    }

    pub fn subsystem_vendor_id(&self) -> u16 {
        self.header.subsystem(self.access()).1
    }

    pub fn set_interrupt_pin(&mut self, pin: u8) {
        self.header
            .update_interrupt(&self.base.root, |(_, line)| (pin, line));
    }

    pub fn set_interrupt_line(&mut self, line: u8) {
        self.header
            .update_interrupt(&self.base.root, |(pin, _)| (pin, line));
    }

    fn access(&self) -> &ConfigAccess {
        &self.base.root
    }

    fn realloc_bar(
        &mut self,
        allocator: &mut SimpleBarAllocator,
    ) -> Result<(), pci_types::BarWriteError> {
        // Disable IO/MEM before reprogramming BARs
        self.base.update_command(|mut cmd| {
            cmd.remove(CommandRegister::IO_ENABLE);
            cmd.remove(CommandRegister::MEMORY_ENABLE);
            cmd
        });
        for (i, bar) in self.bars().into_iter().enumerate() {
            if let Some(bar) = bar {
                match bar {
                    Bar::Memory32 {
                        address: _,
                        size,
                        prefetchable,
                    } => {
                        let addr = allocator.alloc_memory32(size, prefetchable).unwrap();
                        self.set_bar(i as _, addr as usize).unwrap();
                    }
                    Bar::Memory64 {
                        address: _,
                        size,
                        prefetchable,
                    } => {
                        let addr = allocator.alloc_memory64(size, prefetchable).unwrap();
                        self.set_bar(i as _, addr as usize).unwrap();
                    }
                    Bar::Io { port: _ } => {}
                }
            }
        }

        self.base.update_command(|mut cmd| {
            cmd.insert(CommandRegister::MEMORY_ENABLE | CommandRegister::IO_ENABLE);
            cmd
        });

        Ok(())
    }
}

impl Deref for Endpoint {
    type Target = super::PciHeaderBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for Endpoint {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

impl Debug for Endpoint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Endpoint")
            .field("base", &self.base)
            .field("bars", &self.bars())
            .finish()
    }
}

impl Display for Endpoint {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let address = self.base.address();
        let class_info = self.base.revision_and_class();
        let device_type = self.device_type();
        let class_name = format!("{device_type:?}");

        write!(
            f,
            "{:04x}:{:02x}:{:02x}.{} {:<24} {:04x}:{:04x} (rev {:02x}, prog-if {:02x})",
            address.segment(),
            address.bus(),
            address.device(),
            address.function(),
            class_name,
            self.base.vendor_id(),
            self.base.device_id(),
            class_info.revision_id,
            class_info.interface,
        )
    }
}
