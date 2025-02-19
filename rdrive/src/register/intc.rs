use alloc::boxed::Box;
use core::error::Error;

pub use crate::intc::{FdtParseConfigFn, Hardware};
pub use fdt_parser::Node;

pub type OnProbeFdt = fn(node: Node<'_>) -> Result<FdtProbeInfo, Box<dyn Error>>;

pub struct FdtProbeInfo {
    pub hardware: Hardware,
    pub fdt_parse_config_fn: FdtParseConfigFn,
}
