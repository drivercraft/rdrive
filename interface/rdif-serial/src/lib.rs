#![no_std]

extern crate alloc;

use alloc::boxed::Box;

use futures::future::LocalBoxFuture;
use rdif_base::DriverGeneric;

pub type Error = embedded_hal_nb::serial::ErrorKind;

pub trait Sender: Send {
    fn send_blocking(&mut self, data: u8) -> Result<(), Error> {
        spin_on::spin_on(self.send(data))
    }

    fn send(&mut self, data: u8) -> LocalBoxFuture<'_, Result<(), Error>>;
}

pub trait Reciever: Send {
    fn recieve(&mut self) -> LocalBoxFuture<'_, Result<u8, Error>>;
    fn recieve_blocking(&mut self) -> Result<u8, Error> {
        spin_on::spin_on(self.recieve())
    }
}

pub trait Interface: DriverGeneric {
    fn handle_irq(&mut self);
    fn take_split(&mut self) -> Option<(Box<dyn Sender>, Box<dyn Reciever>)>;
    fn restore_split(&mut self, val: (Box<dyn Sender>, Box<dyn Reciever>));
}
