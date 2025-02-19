use core::sync::atomic::{AtomicU64, Ordering};

use crate::custom_id;

custom_id!(DeviceId, u64);
custom_id!(DriverId, u64);

#[derive(Default, Debug, Clone)]
pub struct Descriptor {
    pub device_id: DeviceId,
    pub name: &'static str,
    pub irq_parent: Option<DeviceId>,
}

impl DeviceId {
    pub fn new() -> Self {
        static ITER: AtomicU64 = AtomicU64::new(0);
        Self(ITER.fetch_add(1, Ordering::Relaxed))
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
