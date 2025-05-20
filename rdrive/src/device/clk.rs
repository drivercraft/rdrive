use crate::get_dev;

use super::DeviceId;
use alloc::{collections::btree_map::BTreeMap, string::String};
pub use rdif_clk::*;

pub type Weak = super::DeviceWeak<Hardware>;

#[derive(Debug, Clone)]
pub struct Clock {
    pub id: ClockId,
    pub clk: DeviceId,
    pub freq: Option<u64>,
    pub name: Option<String>,
}

impl Clock {
    pub fn get_dev(&self) -> Option<Weak> {
        get_dev!(self.clk, Clk)
    }
}

pub struct ClockMap {
    pub data: BTreeMap<DeviceId, ClockSrcKind>,
}

pub enum ClockSrcKind {
    OneClk(Clock),
    Multi(BTreeMap<ClockId, Clock>),
}

impl ClockMap {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::new(),
        }
    }
}

impl Default for ClockMap {
    fn default() -> Self {
        Self::new()
    }
}
