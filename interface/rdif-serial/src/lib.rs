#![no_std]

extern crate alloc;

use alloc::boxed::Box;

use rdif_base::DriverGeneric;

pub type Error = embedded_hal_nb::serial::ErrorKind;
pub trait Sender: Send {
    fn can_write(&self) -> bool;
    fn write(&mut self, byte: u8) -> Result<(), Error>;
}

pub trait Serial: DriverGeneric {
    fn split(&mut self) -> Option<(Box<dyn Sender>, Box<dyn Receiver>)>;
}

pub trait Receiver: Send {
    fn can_read(&self) -> bool;
    fn read(&mut self) -> Result<u8, Error>;
}
