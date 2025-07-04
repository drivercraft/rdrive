/// Defines a driver type that wraps a boxed trait object.
///
/// $name: driver name
///
/// $tr: driver trait path
#[macro_export(local_inner_macros)]
macro_rules! def_driver {
    ($name:ident, $tr:path, $($u:path),+) => {
        paste::paste! {
            pub mod [<$name:lower>]{
                pub use $($u),+::*;
                use rdif_base::AsAny;
                use $crate::driver::Class;

                pub struct $name(alloc::boxed::Box<dyn $tr>);

                impl $name {
                    pub fn new<T: $tr>(driver: T) -> Self {
                        Self(alloc::boxed::Box::new(driver))
                    }

                    pub fn typed_ref<T: $tr>(&self) -> Option<&T> {
                        self.raw_any()?.downcast_ref()
                    }

                    pub fn typed_mut<T: $tr>(&mut self) -> Option<&mut T> {
                        self.raw_any_mut()?.downcast_mut()
                    }
                }

                impl rdif_base::DriverGeneric for $name {
                    fn open(&mut self) -> Result<(), rdif_base::KError> {
                        self.0.open()
                    }

                    fn close(&mut self) -> Result<(), rdif_base::KError> {
                        self.0.close()
                    }
                }

                impl Class for $name {
                    fn raw_any(&self) -> Option<&dyn core::any::Any> {
                        Some( <dyn $tr as AsAny>::as_any(self.0.as_ref()))
                    }

                    fn raw_any_mut(&mut self) -> Option<&mut dyn core::any::Any> {
                        Some( <dyn $tr as AsAny>::as_any_mut(self.0.as_mut()))
                    }
                }

                impl core::ops::Deref for $name {
                    type Target = dyn $tr;

                    fn deref(&self) -> &Self::Target {
                        self.0.as_ref()
                    }
                }

                impl core::ops::DerefMut for $name {
                    fn deref_mut(&mut self) -> &mut Self::Target {
                        self.0.as_mut()
                    }
                }

                impl $crate::PlatformDevice {

                    pub fn [<register_ $name:lower>]<T: $tr>(self, driver: T) {
                        self.register($crate::driver::$name::new(driver));
                    }
                }
            }
            pub use [<$name:lower>]::$name;
        }
    };
}

/// Defines a driver type that wraps a boxed trait object.
///
/// $name: driver name
///
/// $t: driver trait path
#[macro_export(local_inner_macros)]
macro_rules! def_driver_rdif {
    ($name:ident) => {
        paste::paste! {
            def_driver!($name, [<rdif_ $name:lower>]::Interface, [<rdif_ $name:lower>]);
        }
    };
}
