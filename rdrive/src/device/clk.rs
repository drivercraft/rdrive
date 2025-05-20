use super::DeviceId;
use alloc::collections::btree_map::BTreeMap;
pub use rdif_clk::*;

pub type Weak = super::DeviceWeak<Hardware>;

#[derive(Clone)]
pub struct ClockIn {
    // pub clk: Weak,
    pub id: ClockId,
    pub name: Option<&'static str>,
}
