use core::cell::UnsafeCell;

use alloc::{
    boxed::Box,
    sync::{Arc, Weak},
};
use rdif_base::{DriverGeneric, KError};

use crate::{
    Config, ConfigError, DataBits, InterruptMask, NotMatchError, Parity, Register, StopBits,
    TransferError,
};

pub struct SerialRaw<T: Register> {
    inner: T,
    tx: Option<SenderRaw<T>>,
    rx: Option<RecieverRaw<T>>,
    handler: Option<IrqHandlerRaw<T>>,
}

impl<T: Register> SerialRaw<T> {
    pub fn new(inner: T) -> Self {
        Self {
            tx: Some(SenderRaw::new(inner.clone())),
            rx: Some(RecieverRaw::new(inner.clone())),
            handler: Some(IrqHandlerRaw::new(inner.clone())),
            inner,
        }
    }

    pub fn set_config(&mut self, config: &Config) -> Result<(), ConfigError> {
        self.inner.set_config(config)
    }

    pub fn baudrate(&self) -> u32 {
        self.inner.baudrate()
    }
    pub fn data_bits(&self) -> DataBits {
        self.inner.data_bits()
    }
    pub fn stop_bits(&self) -> StopBits {
        self.inner.stop_bits()
    }
    pub fn parity(&self) -> Parity {
        self.inner.parity()
    }
    pub fn clock_freq(&self) -> u32 {
        self.inner.clock_freq()
    }

    pub fn enable_loopback(&mut self) {
        self.inner.enable_loopback()
    }
    pub fn disable_loopback(&mut self) {
        self.inner.disable_loopback()
    }
    pub fn is_loopback_enabled(&self) -> bool {
        self.inner.is_loopback_enabled()
    }

    pub fn enable_interrupts(&mut self, mask: InterruptMask) {
        self.inner.set_irq_mask(mask | self.inner.get_irq_mask())
    }
    pub fn disable_interrupts(&mut self, mask: InterruptMask) {
        self.inner.set_irq_mask(self.inner.get_irq_mask() & !mask)
    }

    pub fn get_enabled_interrupts(&self) -> InterruptMask {
        self.inner.get_irq_mask()
    }

    pub fn take_tx(&mut self) -> Option<SenderRaw<T>> {
        self.tx.take()
    }

    pub fn take_rx(&mut self) -> Option<RecieverRaw<T>> {
        self.rx.take()
    }

    pub fn irq_handler(&mut self) -> Option<IrqHandlerRaw<T>> {
        self.handler.take()
    }

    pub fn set_tx(&mut self, tx: SenderRaw<T>) -> Result<(), NotMatchError> {
        if self.inner.get_base() != tx.inner.get_base() {
            return Err(NotMatchError);
        }
        self.tx.replace(tx);
        Ok(())
    }

    pub fn set_rx(&mut self, rx: RecieverRaw<T>) -> Result<(), NotMatchError> {
        if self.inner.get_base() != rx.inner.get_base() {
            return Err(NotMatchError);
        }
        self.rx.replace(rx);
        Ok(())
    }

    pub fn set_irq_handler(&mut self, handler: IrqHandlerRaw<T>) -> Result<(), NotMatchError> {
        if self.inner.get_base() != handler.get_base() {
            return Err(NotMatchError);
        }
        self.handler.replace(handler);
        Ok(())
    }
}

pub struct Serial<T: Register> {
    inner: Arc<UnsafeCell<SerialRaw<T>>>,
}

unsafe impl<T: Register> Send for Serial<T> {}
unsafe impl<T: Register> Sync for Serial<T> {}

impl<T: Register> Serial<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: Arc::new(UnsafeCell::new(SerialRaw::new(inner))),
        }
    }

    fn inner_mut(&mut self) -> &mut SerialRaw<T> {
        unsafe { &mut *self.inner.get() }
    }

    fn inner(&self) -> &SerialRaw<T> {
        unsafe { &*self.inner.get() }
    }

    pub fn open(&mut self) -> Result<(), KError> {
        self.inner_mut().inner.open();
        Ok(())
    }

    pub fn close(&mut self) -> Result<(), KError> {
        self.inner_mut().inner.close();
        Ok(())
    }
}

impl<T: Register> crate::Interface for Serial<T> {
    fn base(&self) -> usize {
        self.inner().inner.get_base()
    }

    fn take_tx(&mut self) -> Option<Box<dyn crate::TSender>> {
        self.inner_mut().take_tx().map(|s| {
            Box::new(Sender {
                inner: Some(s),
                s: Arc::downgrade(&self.inner),
            }) as _
        })
    }

    fn take_rx(&mut self) -> Option<Box<dyn crate::TReciever>> {
        self.inner_mut().take_rx().map(|r| {
            Box::new(Reciever {
                inner: Some(r),
                s: Arc::downgrade(&self.inner),
            }) as _
        })
    }

    fn irq_handler(&mut self) -> Option<Box<dyn crate::TIrqHandler>> {
        self.inner_mut().irq_handler().map(|h| {
            Box::new(IrqHandler {
                inner: Some(h),
                s: Arc::downgrade(&self.inner),
            }) as _
        })
    }

    fn set_config(&mut self, config: &crate::Config) -> Result<(), crate::ConfigError> {
        self.inner_mut().set_config(config)
    }

    fn baudrate(&self) -> u32 {
        self.inner().baudrate()
    }

    fn data_bits(&self) -> crate::DataBits {
        self.inner().data_bits()
    }

    fn stop_bits(&self) -> crate::StopBits {
        self.inner().stop_bits()
    }

    fn parity(&self) -> crate::Parity {
        self.inner().parity()
    }

    fn clock_freq(&self) -> u32 {
        self.inner().clock_freq()
    }

    fn enable_loopback(&mut self) {
        self.inner_mut().enable_loopback()
    }

    fn disable_loopback(&mut self) {
        self.inner_mut().disable_loopback()
    }

    fn is_loopback_enabled(&self) -> bool {
        self.inner().is_loopback_enabled()
    }

    fn enable_interrupts(&mut self, mask: InterruptMask) {
        self.inner_mut().enable_interrupts(mask)
    }

    fn disable_interrupts(&mut self, mask: InterruptMask) {
        self.inner_mut().disable_interrupts(mask)
    }

    fn get_enabled_interrupts(&self) -> InterruptMask {
        self.inner().get_enabled_interrupts()
    }
}

impl<T: Register> DriverGeneric for Serial<T> {
    fn open(&mut self) -> Result<(), KError> {
        self.inner_mut().inner.open();
        Ok(())
    }

    fn close(&mut self) -> Result<(), KError> {
        self.inner_mut().inner.close();
        Ok(())
    }
}

pub struct SenderRaw<T: Register> {
    inner: T,
}

impl<T: Register> SenderRaw<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }
    pub fn send(&mut self, buf: &[u8]) -> usize {
        self.inner.write_buf(buf)
    }
}

pub struct Sender<T: Register> {
    inner: Option<SenderRaw<T>>,
    s: Weak<UnsafeCell<SerialRaw<T>>>,
}

unsafe impl<T: Register> Send for Sender<T> {}

impl<T: Register> Drop for Sender<T> {
    fn drop(&mut self) {
        if let Some(s) = self.s.upgrade() {
            unsafe {
                let s = &mut *s.get();
                let _ = s.set_tx(self.inner.take().unwrap());
            }
        }
    }
}

impl<T: Register> crate::TSender for Sender<T> {
    fn send(&mut self, buf: &[u8]) -> usize {
        self.inner.as_mut().unwrap().send(buf)
    }
}

pub struct RecieverRaw<T: Register> {
    inner: T,
}

impl<T: Register> RecieverRaw<T> {
    fn new(inner: T) -> Self {
        Self { inner }
    }

    pub fn recive(&mut self, buf: &mut [u8]) -> Result<usize, TransferError> {
        let n = self.inner.read_buf(buf)?;
        Ok(n)
    }

    pub fn clean_fifo(&mut self) {
        let mut buff = [0u8; 16];
        while let Ok(n) = self.recive(&mut buff) {
            if n < 16 {
                break;
            }
        }
    }
}

pub struct Reciever<T: Register> {
    inner: Option<RecieverRaw<T>>,
    s: Weak<UnsafeCell<SerialRaw<T>>>,
}

unsafe impl<T: Register> Send for Reciever<T> {}

impl<T: Register> crate::TReciever for Reciever<T> {
    fn recive(&mut self, buf: &mut [u8]) -> Result<usize, TransferError> {
        self.inner.as_mut().unwrap().recive(buf)
    }
    fn clean_fifo(&mut self) {
        self.inner.as_mut().unwrap().clean_fifo();
    }
}

impl<T: Register> Drop for Reciever<T> {
    fn drop(&mut self) {
        if let Some(s) = self.s.upgrade() {
            unsafe {
                let s = &mut *s.get();
                let _ = s.set_rx(self.inner.take().unwrap());
            }
        }
    }
}

pub struct IrqHandlerRaw<T: Register> {
    inner: UnsafeCell<T>,
}

unsafe impl<T: Register> Send for IrqHandlerRaw<T> {}
unsafe impl<T: Register> Sync for IrqHandlerRaw<T> {}

impl<T: Register> IrqHandlerRaw<T> {
    fn new(inner: T) -> Self {
        Self {
            inner: UnsafeCell::new(inner),
        }
    }
    pub fn clean_interrupt_status(&self) -> InterruptMask {
        unsafe { (*self.inner.get()).clean_interrupt_status() }
    }

    fn get_base(&self) -> usize {
        unsafe { (*self.inner.get()).get_base() }
    }
}

pub struct IrqHandler<T: Register> {
    inner: Option<IrqHandlerRaw<T>>,
    s: Weak<UnsafeCell<SerialRaw<T>>>,
}
unsafe impl<T: Register> Send for IrqHandler<T> {}
unsafe impl<T: Register> Sync for IrqHandler<T> {}

impl<T: Register> crate::TIrqHandler for IrqHandler<T> {
    fn clean_interrupt_status(&self) -> InterruptMask {
        self.inner.as_ref().unwrap().clean_interrupt_status()
    }
}

impl<T: Register> Drop for IrqHandler<T> {
    fn drop(&mut self) {
        if let Some(s) = self.s.upgrade() {
            unsafe {
                let s = &mut *s.get();
                let _ = s.set_irq_handler(self.inner.take().unwrap());
            }
        }
    }
}
