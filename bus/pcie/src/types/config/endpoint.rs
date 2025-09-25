use core::{
    fmt::{Debug, Display},
    ops::{Deref, DerefMut, Range},
};

use alloc::vec::Vec;
use pci_types::{
    capability::PciCapability, device_type::DeviceType, Bar, CommandRegister, ConfigRegionAccess,
    EndpointHeader, PciAddress,
};
use rdif_pcie::ConfigAccess;

use crate::{BarHeader, BarVec, SimpleBarAllocator};

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

    pub fn bar(&self, index: usize) -> Option<Range<usize>> {
        assert!(index < 6, "BAR index out of range");
        let bars = self.bars();
        let r = match &bars {
            BarVec::Memory32(bar_vec) => {
                let b = bar_vec.get(index)?;
                b.address as usize..(b.address as usize + b.size as usize)
            }
            BarVec::Memory64(bar_vec) => {
                let b = bar_vec.get(index)?;
                b.address as usize..(b.address + b.size) as usize
            }
            BarVec::Io(_) => unimplemented!(), // IO BAR size is typically 4 bytes
        };
        Some(r)
    }

    pub fn bars(&self) -> BarVec {
        self.header.parse_bar(6, &self.base.root)
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
        let bar = self.bars();

        match &bar {
            crate::BarVec::Memory32(bar_vec) => {
                // Compute new values with mutable allocator, then write using immutable access
                let new_vals = {
                    bar_vec
                        .iter()
                        .map(|old| {
                            old.clone().map(|ref b| {
                                allocator
                                    .alloc_memory32_with_pref(b.size, b.prefetchable)
                                    .unwrap()
                            })
                        })
                        .collect::<alloc::vec::Vec<_>>()
                };
                for (i, v) in new_vals.into_iter().enumerate() {
                    if let Some(value) = v {
                        bar_vec.set(i, value, &self.base.root).unwrap();
                    }
                }
                self.base.update_command(|mut cmd| {
                    cmd.insert(CommandRegister::MEMORY_ENABLE);
                    cmd
                });
            }
            crate::BarVec::Memory64(bar_vec) => {
                let new_vals = {
                    bar_vec
                        .iter()
                        .map(|old| {
                            old.clone().map(|ref b| {
                                if b.address > 0 && b.address < u32::MAX as u64 {
                                    allocator
                                        .alloc_memory32_with_pref(b.size as u32, b.prefetchable)
                                        .unwrap() as u64
                                } else {
                                    allocator
                                        .alloc_memory64_with_pref(b.size, b.prefetchable)
                                        .unwrap()
                                }
                            })
                        })
                        .collect::<alloc::vec::Vec<_>>()
                };
                for (i, v) in new_vals.into_iter().enumerate() {
                    if let Some(value) = v {
                        bar_vec
                            .set(i, value, &self.base.root)
                            .inspect_err(|e| error!("{e:?}"))
                            .unwrap();
                    }
                }
                self.base.update_command(|mut cmd| {
                    cmd.insert(CommandRegister::MEMORY_ENABLE);
                    cmd
                });
            }
            crate::BarVec::Io(_bar_vec_t) => {
                self.base.update_command(|mut cmd| {
                    cmd.insert(CommandRegister::IO_ENABLE);
                    cmd
                });
            }
        }

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

impl BarHeader for EndpointHeader {
    fn read_bar<A: ConfigRegionAccess>(&self, slot: usize, access: &A) -> Option<Bar> {
        self.bar(slot as u8, access)
    }

    fn address(&self) -> PciAddress {
        self.header().address()
    }

    fn header_type(&self) -> pci_types::HeaderType {
        pci_types::HeaderType::Endpoint
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
