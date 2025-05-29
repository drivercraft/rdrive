use core::{future, task::Poll};

use alloc::boxed::Box;
pub use async_trait::async_trait;
pub use rdif_def::io::*;

#[async_trait(?Send)]
pub trait Read {
    /// Read data from the device.
    fn read(&mut self, buf: &mut [u8]) -> Result;

    /// Read data from the device, blocking until all bytes are read
    fn read_all_blocking(&mut self, buf: &mut [u8]) -> Result {
        let mut n = 0;
        while n < buf.len() {
            let tmp = &mut buf[n..];
            if let Err(mut e) = self.read(tmp) {
                n += e.success_pos;
                if matches!(e.kind, ErrorKind::Interrupted) {
                    continue;
                } else {
                    e.success_pos = n;
                    return Err(e);
                }
            } else {
                n += tmp.len();
            }
        }

        Ok(())
    }

    async fn read_all(&mut self, buf: &mut [u8]) -> Result {
        let mut n = 0;
        future::poll_fn(move |cx| {
            let tmp = &mut buf[n..];
            if let Err(mut e) = self.read(tmp) {
                n += e.success_pos;
                if !matches!(e.kind, ErrorKind::Interrupted) {
                    e.success_pos = n;
                    return Poll::Ready(Err(e));
                }
            } else {
                n += tmp.len();
            }
            if n == buf.len() {
                Poll::Ready(Ok(()))
            } else {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
        .await
    }
}

#[async_trait(?Send)]
pub trait Write {
    /// Write data to the device.
    fn write(&mut self, buf: &[u8]) -> Result;

    fn write_all_blocking(&mut self, buf: &[u8]) -> Result {
        let mut n = 0;
        while n < buf.len() {
            let tmp = &buf[n..];
            if let Err(mut e) = self.write(tmp) {
                n += e.success_pos;
                if matches!(e.kind, ErrorKind::Interrupted) {
                    continue;
                } else {
                    e.success_pos = n;
                    return Err(e);
                }
            } else {
                n += tmp.len();
            }
        }
        Ok(())
    }

    async fn write_all(&mut self, buf: &[u8]) -> Result {
        let mut n = 0;
        future::poll_fn(move |cx| {
            let tmp = &buf[n..];
            if let Err(mut e) = self.write(tmp) {
                n += e.success_pos;
                if !matches!(e.kind, ErrorKind::Interrupted) {
                    e.success_pos = n;
                    return Poll::Ready(Err(e));
                }
            } else {
                n += tmp.len();
            }
            if n == buf.len() {
                Poll::Ready(Ok(()))
            } else {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
        })
        .await
    }
}

#[cfg(test)]
mod test {

    use super::*;

    struct TRead;

    #[async_trait(?Send)]
    impl Read for TRead {
        fn read(&mut self, buf: &mut [u8]) -> Result {
            const MAX: usize = 2;
            if !buf.is_empty() {
                buf[0] = 1;
            }
            if buf.len() > 1 {
                buf[1] = 1;
            }
            if buf.len() > MAX {
                return Err(Error {
                    kind: ErrorKind::Interrupted,
                    success_pos: MAX,
                });
            }
            Ok(())
        }
    }
    struct TWrite {
        data: [u8; 8],
        iter: usize,
    }

    impl TWrite {
        fn new() -> Self {
            Self {
                data: [0; 8],
                iter: 0,
            }
        }

        fn put(&mut self, data: u8) -> core::result::Result<(), ErrorKind> {
            if self.iter >= self.data.len() {
                return Err(ErrorKind::BrokenPipe);
            }
            self.data[self.iter] = data;
            self.iter += 1;
            Ok(())
        }
    }

    impl Write for TWrite {
        fn write(&mut self, buf: &[u8]) -> Result {
            const MAX: usize = 2;
            for (n, i) in (0..MAX.min(buf.len())).enumerate() {
                self.put(buf[i]).map_err(|e| Error {
                    kind: e,
                    success_pos: n,
                })?;
            }
            if buf.len() > MAX {
                return Err(Error {
                    kind: ErrorKind::Interrupted,
                    success_pos: MAX,
                });
            }

            Ok(())
        }
    }

    #[test]
    fn test_r() {
        let mut buf = [0; 8];
        let mut read = TRead;
        read.read_all_blocking(&mut buf).unwrap();

        assert_eq!(buf, [1; 8]);
    }

    #[tokio::test]
    async fn test_async_r() {
        let mut buf = [0; 8];

        let mut read = TRead;
        read.read_all(&mut buf).await.unwrap();

        assert_eq!(buf, [1; 8]);
    }

    #[test]
    fn test_w() {
        let buf = [1; 8];
        let mut w = TWrite::new();
        w.write_all_blocking(&buf).unwrap();

        assert_eq!(buf, w.data);
    }

    #[tokio::test]
    async fn test_async_w() {
        let buf = [1; 8];
        let mut w = TWrite::new();
        w.write_all(&buf).await.unwrap();

        assert_eq!(buf, w.data);
    }
}
