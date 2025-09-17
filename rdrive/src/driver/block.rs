use crate::driver::Class;

pub use rdif_block::*;

impl Class for Block {
    fn raw_any(&self) -> Option<&dyn core::any::Any> {
        Block::raw_any(self)
    }

    fn raw_any_mut(&mut self) -> Option<&mut dyn core::any::Any> {
        Block::raw_any_mut(self)
    }
}

impl crate::PlatformDevice {
    pub fn register_block<T: rdif_block::Interface>(self, driver: T) {
        self.register(Block::new(driver));
    }
}
