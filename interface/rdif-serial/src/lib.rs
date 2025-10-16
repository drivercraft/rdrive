#![no_std]

extern crate alloc;

use core::any::Any;

use alloc::boxed::Box;
use bitflags::bitflags;
use mbarrier::rmb;

pub use rdif_base::{DriverGeneric, KError};

pub type BIrqHandler = Box<dyn TIrqHandler>;
pub type BSender = Box<dyn TSender>;
pub type BReciever = Box<dyn TReciever>;
pub type BSerial = Box<dyn Interface>;

mod serial;

pub use serial::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigError {
    /// 无效的波特率
    InvalidBaudrate,
    /// 不支持的数据位配置
    UnsupportedDataBits,
    /// 不支持的停止位配置
    UnsupportedStopBits,
    /// 不支持的奇偶校验配置
    UnsupportedParity,
    /// 寄存器访问错误
    RegisterError,
    /// 超时错误
    Timeout,
}

#[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferError {
    #[error("Data overrun by `{0:#x}`")]
    Overrun(u8),
    #[error("Parity error")]
    Parity,
    #[error("Framing error")]
    Framing,
    #[error("Break condition")]
    Break,
    #[error("Serial closed")]
    Closed,
}

/// 数据位配置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DataBits {
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
}

/// 停止位配置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum StopBits {
    One = 1,
    Two = 2,
}

/// 奇偶校验配置
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parity {
    None,
    Even,
    Odd,
    Mark,
    Space,
}

bitflags! {
    /// 中断状态标志
    #[derive(Debug, Clone, Copy)]
    pub struct InterruptMask: u32 {
        /// received data, including error data
        const RX_AVAILABLE = 0x01;
        const TX_EMPTY = 0x02;
    }
}

impl InterruptMask {
    pub fn rx_available(&self) -> bool {
        self.contains(InterruptMask::RX_AVAILABLE)
    }

    pub fn tx_empty(&self) -> bool {
        self.contains(InterruptMask::TX_EMPTY)
    }
}

bitflags! {
    /// 线路状态标志
    #[derive(Debug, Clone, Copy)]
    pub struct LineStatus: u32 {
        const DATA_READY = 0x01;
        const TX_HOLDING_EMPTY = 0x20;
    }
}

impl LineStatus {
    pub fn can_read(&self) -> bool {
        self.contains(LineStatus::DATA_READY)
    }

    pub fn can_write(&self) -> bool {
        self.contains(LineStatus::TX_HOLDING_EMPTY)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub baudrate: Option<u32>,
    pub data_bits: Option<DataBits>,
    pub stop_bits: Option<StopBits>,
    pub parity: Option<Parity>,
}

impl Config {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn baudrate(mut self, baudrate: u32) -> Self {
        self.baudrate = Some(baudrate);
        self
    }

    pub fn data_bits(mut self, data_bits: DataBits) -> Self {
        self.data_bits = Some(data_bits);
        self
    }

    pub fn stop_bits(mut self, stop_bits: StopBits) -> Self {
        self.stop_bits = Some(stop_bits);
        self
    }

    pub fn parity(mut self, parity: Parity) -> Self {
        self.parity = Some(parity);
        self
    }
}

pub trait Register: Send + Sync + Any + 'static {
    // ==================== 基础数据传输 ====================
    fn write_byte(&mut self, byte: u8);
    fn read_byte(&mut self) -> Result<u8, TransferError>;

    // ==================== 配置管理 ====================
    fn set_config(&mut self, config: &Config) -> Result<(), ConfigError>;

    fn baudrate(&self) -> u32;
    fn data_bits(&self) -> DataBits;
    fn stop_bits(&self) -> StopBits;
    fn parity(&self) -> Parity;
    fn clock_freq(&self) -> u32;

    fn open(&mut self);
    fn close(&mut self);

    // ==================== 回环控制 ====================
    /// 启用回环模式
    fn enable_loopback(&mut self);
    /// 禁用回环模式
    fn disable_loopback(&mut self);
    /// 检查回环模式是否启用
    fn is_loopback_enabled(&self) -> bool;

    // ==================== 中断管理 ====================
    /// 设置中断使能掩码
    fn set_irq_mask(&mut self, mask: InterruptMask);
    /// 获取当前中断使能掩码
    fn get_irq_mask(&self) -> InterruptMask;
    /// 获取并清除所有中断状态
    fn clean_interrupt_status(&mut self) -> InterruptMask;

    // ==================== 传输状态查询 ====================

    /// 获取线路状态
    fn line_status(&mut self) -> LineStatus;

    // ==================== 底层寄存器访问 ====================
    /// 直接读取寄存器
    fn read_reg(&self, offset: usize) -> u32;
    /// 直接写入寄存器
    fn write_reg(&mut self, offset: usize, value: u32);

    fn get_base(&self) -> usize;
    fn set_base(&mut self, base: usize);

    fn read_buf(&mut self, buf: &mut [u8]) -> Result<usize, TransferError> {
        let mut read_count = 0;
        for byte in buf.iter_mut() {
            if !self.line_status().can_read() {
                break;
            }
            rmb();
            match self.read_byte() {
                Ok(b) => *byte = b,
                Err(e) => return Err(e),
            }

            read_count += 1;
        }

        Ok(read_count)
    }

    fn write_buf(&mut self, buf: &[u8]) -> usize {
        let mut write_count = 0;
        for &byte in buf.iter() {
            if !self.line_status().can_write() {
                break;
            }
            rmb();
            self.write_byte(byte);
            write_count += 1;
        }
        write_count
    }
}

pub trait Interface: DriverGeneric {
    fn irq_handler(&mut self) -> Option<Box<dyn TIrqHandler>>;
    fn take_tx(&mut self) -> Option<Box<dyn TSender>>;
    fn take_rx(&mut self) -> Option<Box<dyn TReciever>>;
    /// Base address of the serial port
    fn base(&self) -> usize;
    /// Set base address of the serial port
    fn set_base(&mut self, base: usize);

    fn set_config(&mut self, config: &Config) -> Result<(), ConfigError>;

    fn baudrate(&self) -> u32;
    fn data_bits(&self) -> DataBits;
    fn stop_bits(&self) -> StopBits;
    fn parity(&self) -> Parity;
    fn clock_freq(&self) -> u32;

    fn enable_loopback(&mut self);
    fn disable_loopback(&mut self);
    fn is_loopback_enabled(&self) -> bool;

    fn enable_interrupts(&mut self, mask: InterruptMask);
    fn disable_interrupts(&mut self, mask: InterruptMask);
    fn get_enabled_interrupts(&self) -> InterruptMask;
}

pub trait TIrqHandler: Send + Sync + 'static {
    fn clean_interrupt_status(&self) -> InterruptMask;
}

pub trait TSender: Send + 'static {
    /// Send data from buf, return sent bytes. If return bytes is less than buf.len(), it means no more space, need to retry later.
    fn send(&mut self, buf: &[u8]) -> Result<usize, TransferError>;
}

pub trait TReciever: Send + 'static {
    /// Recv data into buf, return recv bytes. If return bytes is less than buf.len(), it means no more data.
    fn recive(&mut self, buf: &mut [u8]) -> Result<usize, TransferError>;

    fn clean_fifo(&mut self);
}
