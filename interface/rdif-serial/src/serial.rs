use core::{cell::UnsafeCell, sync::atomic::AtomicBool};

use alloc::{
    boxed::Box,
    sync::{Arc, Weak},
};
use rdif_base::{DriverGeneric, KError};

use crate::{InterruptMask, Register, TransferError};

pub struct Serial<T: Register> {
    inner: Arc<UnsafeCell<T>>,
    is_tx_taken: Arc<AtomicBool>,
    is_rx_taken: Arc<AtomicBool>,
    is_irq_handler_taken: Arc<AtomicBool>,
}

unsafe impl<T: Register> Send for Serial<T> {}
unsafe impl<T: Register> Sync for Serial<T> {}

impl<T: Register> Serial<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: Arc::new(UnsafeCell::new(inner)),
            is_tx_taken: Arc::new(AtomicBool::new(false)),
            is_rx_taken: Arc::new(AtomicBool::new(false)),
            is_irq_handler_taken: Arc::new(AtomicBool::new(false)),
        }
    }

    fn inner_mut(&mut self) -> &mut T {
        unsafe { &mut *self.inner.get() }
    }

    fn inner(&self) -> &T {
        unsafe { &*self.inner.get() }
    }

    pub fn open(&mut self) -> Result<(), KError> {
        self.inner_mut().open();
        Ok(())
    }

    pub fn close(&mut self) -> Result<(), KError> {
        self.inner_mut().close();
        Ok(())
    }
}

impl<T: Register> crate::Interface for Serial<T> {
    fn base(&self) -> usize {
        self.inner().get_base()
    }

    fn set_base(&mut self, base: usize) {
        self.inner_mut().set_base(base);
    }

    fn take_tx(&mut self) -> Option<Box<dyn crate::TSender>> {
        if self
            .is_tx_taken
            .swap(true, core::sync::atomic::Ordering::SeqCst)
        {
            return None;
        }
        Some(Box::new(Sender {
            s: Arc::downgrade(&self.inner),
            b: self.is_tx_taken.clone(),
        }) as _)
    }

    fn take_rx(&mut self) -> Option<Box<dyn crate::TReciever>> {
        if self
            .is_rx_taken
            .swap(true, core::sync::atomic::Ordering::SeqCst)
        {
            return None;
        }
        Some(Box::new(Reciever {
            s: Arc::downgrade(&self.inner),
            b: self.is_rx_taken.clone(),
        }) as _)
    }

    fn irq_handler(&mut self) -> Option<Box<dyn crate::TIrqHandler>> {
        if self
            .is_irq_handler_taken
            .swap(true, core::sync::atomic::Ordering::SeqCst)
        {
            return None;
        }
        Some(Box::new(IrqHandler {
            s: self.inner.clone(),
            b: self.is_irq_handler_taken.clone(),
        }) as _)
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
        let mut val = self.inner().get_irq_mask();
        val |= mask;
        self.inner_mut().set_irq_mask(val);
    }

    fn disable_interrupts(&mut self, mask: InterruptMask) {
        let mut val = self.inner().get_irq_mask();
        val &= !mask;
        self.inner_mut().set_irq_mask(val);
    }

    fn get_enabled_interrupts(&self) -> InterruptMask {
        self.inner().get_irq_mask()
    }
}

impl<T: Register> DriverGeneric for Serial<T> {
    fn open(&mut self) -> Result<(), KError> {
        self.inner_mut().open();
        Ok(())
    }

    fn close(&mut self) -> Result<(), KError> {
        self.inner_mut().close();
        Ok(())
    }
}

pub struct Sender<T: Register> {
    s: Weak<UnsafeCell<T>>,
    b: Arc<AtomicBool>,
}

unsafe impl<T: Register> Send for Sender<T> {}

impl<T: Register> Drop for Sender<T> {
    fn drop(&mut self) {
        self.b.store(false, core::sync::atomic::Ordering::SeqCst);
    }
}

impl<T: Register> crate::TSender for Sender<T> {
    fn send(&mut self, buf: &[u8]) -> Result<usize, TransferError> {
        let s = self.s.upgrade().ok_or(TransferError::SerialReleased)?;
        let s = unsafe { &mut *s.get() };
        Ok(s.write_buf(buf))
    }
}

pub struct Reciever<T: Register> {
    s: Weak<UnsafeCell<T>>,
    b: Arc<AtomicBool>,
}

unsafe impl<T: Register> Send for Reciever<T> {}

impl<T: Register> crate::TReciever for Reciever<T> {
    fn recive(&mut self, buf: &mut [u8]) -> Result<usize, TransferError> {
        let s = self.s.upgrade().ok_or(TransferError::SerialReleased)?;
        let s = unsafe { &mut *s.get() };
        Ok(s.read_buf(buf)?)
    }
    fn clean_fifo(&mut self) {
        let mut buff = [0u8; 16];
        while let Ok(n) = self.recive(&mut buff) {
            if n < 16 {
                break;
            }
        }
    }
}

impl<T: Register> Drop for Reciever<T> {
    fn drop(&mut self) {
        self.b.store(false, core::sync::atomic::Ordering::SeqCst);
    }
}

pub struct IrqHandler<T: Register> {
    s: Arc<UnsafeCell<T>>,
    b: Arc<AtomicBool>,
}
unsafe impl<T: Register> Send for IrqHandler<T> {}
unsafe impl<T: Register> Sync for IrqHandler<T> {}

impl<T: Register> crate::TIrqHandler for IrqHandler<T> {
    fn clean_interrupt_status(&self) -> InterruptMask {
        let s = unsafe { &mut *self.s.get() };
        s.clean_interrupt_status()
    }
}

impl<T: Register> Drop for IrqHandler<T> {
    fn drop(&mut self) {
        self.b.store(false, core::sync::atomic::Ordering::SeqCst);
    }
}
