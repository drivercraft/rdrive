use alloc::collections::btree_map::BTreeMap;
pub use rdif_clk::*;
use super::DeviceId;

pub type Weak = super::DeviceWeak<Hardware>;

#[derive( Clone)]
pub struct ClockIn {
    pub clk: Weak,
    pub id: ClockId,
    pub name: Option<&'static str>,
}


