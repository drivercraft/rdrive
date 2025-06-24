use rdif_base::DriverGeneric;

#[macro_use]
mod _macros;

pub struct Empty;

impl DriverGeneric for Empty {
    fn open(&mut self) -> Result<(), rdif_base::KError> {
        Ok(())
    }

    fn close(&mut self) -> Result<(), rdif_base::KError> {
        Ok(())
    }
}

def_driver_rdif!(Intc);
def_driver_rdif!(Clk);
def_driver_rdif!(Power);
def_driver_rdif!(Systick);
def_driver_rdif!(Serial);
