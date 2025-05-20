use core::sync::atomic::{AtomicU64, Ordering};

pub use alloc::vec::Vec;
pub use rdif_base::*;

use crate::custom_id;

use super::clk::Clock;

custom_id!(DeviceId, u64);
custom_id!(DriverId, u64);

#[derive(Default, Debug, Clone)]
pub struct Descriptor {
    pub device_id: DeviceId,
    pub name: &'static str,
    pub irq_parent: Option<DeviceId>,
    pub irqs: Vec<IrqConfig>,
    pub clocks: Vec<Clock>,
}

impl Descriptor {
    pub fn get_clk_by_name(&self, name: &str) -> Option<Clock> {
        for clk in self.clocks.iter() {
            if let Some(n) = clk.name.as_ref() {
                if n.eq(name) {
                    return Some(clk.clone());
                }
            }
        }
        None
    }
}

static ITER: AtomicU64 = AtomicU64::new(0);

impl DeviceId {
    pub fn new() -> Self {
        Self(ITER.fetch_add(1, Ordering::SeqCst))
    }
}

macro_rules! impl_driver_id_for {
    ($t:ty) => {
        impl From<$t> for DriverId {
            fn from(value: $t) -> Self {
                Self(value as _)
            }
        }
    };
}

impl_driver_id_for!(usize);
impl_driver_id_for!(u32);
