pub use rdif_def::io::*;

pub trait Read {
    /// Read data from the device. Returns the number of bytes read.
    fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

    /// Read data from the device, blocking until all bytes are read
    fn read_all_blocking(&mut self, mut buf: &mut [u8]) -> Result {
        let mut n = 0;
        while n < buf.len() {
            match self.read(buf) {
                Ok(m) => {
                    n += m;
                    buf = &mut buf[m..];
                }
                Err(e) => {
                    if let Error::Interrupted = e {
                        continue;
                    } else {
                        Err(e)?
                    }
                }
            };
        }
        Ok(())
    }
}

pub trait Write {
    /// Write data to the device. Returns the number of bytes written.
    fn write(&mut self, buf: &[u8]) -> Result<usize>;

    fn write_all_blocking(&mut self, buf: &[u8]) -> Result {
        let mut n = 0;
        while n < buf.len() {
            match self.write(&buf[n..]) {
                Ok(m) => {
                    n += m;
                }
                Err(e) => {
                    if let Error::Interrupted = e {
                        continue;
                    } else {
                        Err(e)?
                    }
                }
            }
        }
        Ok(())
    }
}
