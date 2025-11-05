use eyre::Result;

use crate::{
    jtag,
    units::{Bits, Bytes},
};

/// A device that is able to talk over JTAG.
///
/// The implementation is allowed to flush at any time, though only required to
/// flush on [`Backend::flush`].
///
/// `last` refers to whether the given packet is the last packet before
/// transitioning states.
pub trait Backend {
    fn tms(&mut self, buf: &mut dyn Buffer, path: jtag::Path) -> Result<()>;

    fn tdi_bytes(&mut self, buf: &mut dyn Buffer, tdi: &[u8], last: bool) -> Result<()>;
    fn tdi_bits(
        &mut self,
        buf: &mut dyn Buffer,
        tdi: u8,
        len: Bits<usize>,
        last: bool,
    ) -> Result<()>;

    fn tdo_bytes(&mut self, buf: &mut dyn Buffer, len: Bytes<usize>, last: bool) -> Result<()>;

    fn tdi_tdo_bytes(&mut self, buf: &mut dyn Buffer, tdi: &[u8], last: bool) -> Result<()>;

    /// Run any queud IO commands
    fn flush(&mut self, buf: &mut dyn Buffer) -> Result<()>;
}

pub trait Buffer {
    /// Extend the buffer, returning a mutable slice to the newly-allocated
    /// memory
    fn extend(&mut self, size: usize) -> &mut [u8];
}

impl<B: Backend + ?Sized> Backend for Box<B> {
    fn tms(&mut self, buf: &mut dyn Buffer, path: jtag::Path) -> Result<()> {
        B::tms(&mut *self, buf, path)
    }

    fn tdi_bytes(&mut self, buf: &mut dyn Buffer, tdi: &[u8], last: bool) -> Result<()> {
        B::tdi_bytes(&mut *self, buf, tdi, last)
    }

    fn tdi_bits(
        &mut self,
        buf: &mut dyn Buffer,
        tdi: u8,
        len: Bits<usize>,
        last: bool,
    ) -> Result<()> {
        B::tdi_bits(&mut *self, buf, tdi, len, last)
    }

    fn tdo_bytes(&mut self, buf: &mut dyn Buffer, len: Bytes<usize>, last: bool) -> Result<()> {
        B::tdo_bytes(&mut *self, buf, len, last)
    }

    fn tdi_tdo_bytes(&mut self, buf: &mut dyn Buffer, tdi: &[u8], last: bool) -> Result<()> {
        B::tdi_tdo_bytes(&mut *self, buf, tdi, last)
    }

    fn flush(&mut self, buf: &mut dyn Buffer) -> Result<()> {
        B::flush(&mut *self, buf)
    }
}

impl Buffer for Vec<u8> {
    fn extend(&mut self, size: usize) -> &mut [u8] {
        let len = self.len();
        self.resize(len + size, 0);
        &mut self[len..]
    }
}
