#![no_std]

extern crate alloc;

use alloc::sync::Arc;

use rdif_base::DriverGeneric;

pub type Error = embedded_hal_nb::serial::ErrorKind;

pub trait Sender: Send {
    fn can_write(&self) -> bool;
    fn write(&mut self, byte: u8) -> Result<(), Error>;
    fn wrire_bytes(&mut self, bytes: &[u8]) -> Result<usize, Error> {
        let mut n = 0;
        for &byte in bytes {
            if !self.can_write() {
                break;
            }
            self.write(byte)?;
            n += 1;
        }
        Ok(n)
    }
}

pub trait Serial: DriverGeneric {
    fn can_write(&self) -> bool;
    fn write(&mut self, byte: u8) -> Result<(), Error>;
    fn wrire_bytes(&mut self, bytes: &[u8]) -> Result<usize, Error> {
        let mut n = 0;
        for &byte in bytes {
            if !self.can_write() {
                break;
            }
            self.write(byte)?;
            n += 1;
        }
        Ok(n)
    }

    fn can_read(&self) -> bool;
    fn read(&mut self) -> Result<u8, Error>;
    fn read_bytes(&mut self, bytes: &mut [u8]) -> Result<usize, Error> {
        let mut n = 0;
        for byte in bytes.iter_mut() {
            if !self.can_read() {
                break;
            }
            *byte = self.read()?;
            n += 1;
        }
        Ok(n)
    }
}

pub struct DeviceSerial {
    inner: Arc<dyn Serial>,
    tx: Option<Arc<dyn Serial>>,
    rx: Option<Arc<dyn Serial>>,
}
