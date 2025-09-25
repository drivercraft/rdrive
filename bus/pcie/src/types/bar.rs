use core::{fmt::Debug, ops::Index};

use alloc::vec::Vec;
use pci_types::{
    Bar, BarWriteError, ConfigRegionAccess, EndpointHeader, HeaderType, PciAddress, PciHeader,
};

#[derive(Clone)]
pub enum BarVec {
    Memory32(BarVecT<Bar32>),
    Memory64(BarVecT<Bar64>),
    Io(BarVecT<BarIO>),
}

impl Debug for BarVec {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Memory32(arg0) => write!(f, "{arg0:?}"),
            Self::Memory64(arg0) => write!(f, "{arg0:?}"),
            Self::Io(arg0) => write!(f, "{arg0:?}"),
        }
    }
}

#[derive(Clone)]
pub struct Bar64 {
    pub address: u64,
    pub size: u64,
    pub prefetchable: bool,
}

#[derive(Clone)]
pub struct Bar32 {
    pub address: u32,
    pub size: u32,
    pub prefetchable: bool,
}

#[derive(Debug, Clone)]
pub struct BarIO {
    pub port: u32,
}

pub(crate) trait BarHeader: Sized {
    fn read_bar<A: ConfigRegionAccess>(&self, slot: usize, access: &A) -> Option<Bar>;

    fn address(&self) -> PciAddress;

    fn header_type(&self) -> HeaderType;

    fn parse_bar<A: ConfigRegionAccess>(&self, slot_size: usize, access: &A) -> BarVec {
        let bar0 = match self.read_bar(0, access) {
            Some(bar0) => bar0,
            None => {
                return BarVec::Memory32(BarVecT {
                    data: Vec::new(),
                    address: self.address(),
                    header_type: self.header_type(),
                })
            }
        };

        match bar0 {
            Bar::Memory32 {
                address,
                size,
                prefetchable,
            } => {
                let mut v = alloc::vec![None; slot_size];
                v[0] = Some(Bar32 {
                    address,
                    size,
                    prefetchable,
                });

                (1..slot_size).for_each(|i| {
                    if let Some(Bar::Memory32 {
                        address,
                        size,
                        prefetchable,
                    }) = self.read_bar(i, access)
                    {
                        v[i] = Some(Bar32 {
                            address,
                            size,
                            prefetchable,
                        });
                    }
                });

                BarVec::Memory32(BarVecT {
                    data: v,
                    address: self.address(),
                    header_type: self.header_type(),
                })
            }
            Bar::Memory64 {
                address,
                size,
                prefetchable,
            } => {
                let mut v = alloc::vec![None; slot_size/2];
                v[0] = Some(Bar64 {
                    address,
                    size,
                    prefetchable,
                });

                (1..slot_size / 2).for_each(|i| {
                    if let Some(Bar::Memory64 {
                        address,
                        size,
                        prefetchable,
                    }) = self.read_bar(i * 2, access)
                    {
                        v[i] = Some(Bar64 {
                            address,
                            size,
                            prefetchable,
                        });
                    }
                });
                BarVec::Memory64(BarVecT {
                    data: v,
                    address: self.address(),
                    header_type: self.header_type(),
                })
            }
            Bar::Io { port } => {
                let mut v = alloc::vec![None; slot_size];

                v[0] = Some(BarIO { port });

                (1..slot_size).for_each(|i| {
                    if let Some(Bar::Io { port }) = self.read_bar(i, access) {
                        v[i] = Some(BarIO { port });
                    }
                });

                BarVec::Io(BarVecT {
                    data: v,
                    address: self.address(),
                    header_type: self.header_type(),
                })
            }
        }
    }
}

impl Debug for Bar32 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Memory32 {{ address: {:#p}, size: {:#x}, prefetchable: {} }}",
            self.address as *const u8, self.size, self.prefetchable
        )
    }
}

impl Debug for Bar64 {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Memory64 {{ address: {:#p}, size: {:#x}, prefetchable: {} }}",
            self.address as *const u8, self.size, self.prefetchable
        )
    }
}

#[derive(Clone)]
pub struct BarVecT<T> {
    data: Vec<Option<T>>,
    address: PciAddress,
    header_type: pci_types::HeaderType,
}

impl<T: Debug> Debug for BarVecT<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for (i, bar) in self.data.iter().enumerate() {
            if let Some(bar) = bar {
                writeln!(f, "BAR{i}: {bar:?}")?;
            }
        }
        Ok(())
    }
}

impl BarVecT<Bar32> {
    pub(crate) fn set<A: ConfigRegionAccess>(
        &self,
        index: usize,
        value: u32,
        access: &A,
    ) -> core::result::Result<(), BarWriteError> {
        let header = PciHeader::new(self.address);
        match self.header_type {
            pci_types::HeaderType::PciPciBridge => {
                todo!()
            }
            pci_types::HeaderType::Endpoint => unsafe {
                EndpointHeader::from_header(header, access)
                    .unwrap()
                    .write_bar(index as _, access, value as _)
            },
            _ => panic!("Invalid header type"),
        }
    }
}

impl BarVecT<Bar64> {
    pub(crate) fn set<A: ConfigRegionAccess>(
        &self,
        index: usize,
        value: u64,
        access: &A,
    ) -> core::result::Result<(), BarWriteError> {
        let header = PciHeader::new(self.address);
        match self.header_type {
            pci_types::HeaderType::PciPciBridge => {
                todo!()
            }
            pci_types::HeaderType::Endpoint => unsafe {
                EndpointHeader::from_header(header, access)
                    .unwrap()
                    .write_bar((index * 2) as _, access, value as _)
            },
            _ => panic!("Invalid header type"),
        }
    }
}

impl<T> Index<usize> for BarVecT<T> {
    type Output = Option<T>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl<T> BarVecT<T> {
    pub fn iter(&self) -> impl Iterator<Item = &Option<T>> {
        self.data.iter()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.data.get(index).and_then(|v| v.as_ref())
    }
}
