use eyre::Result;

use crate::{
    jtag,
    units::{Bits, Bytes},
};

pub enum Data<'d> {
    Tx(&'d [u8]),
    Rx(Bytes<usize>),
    TxRx(&'d [u8]),
    ConstantTx(bool, Bytes<usize>),
}

/// A device that is able to talk over JTAG.
///
/// The implementation is allowed to flush at any time, though only required to
/// flush on [`Backend::flush`].
#[async_trait::async_trait]
pub trait Backend: Send {
    async fn tms(&mut self, buf: &mut dyn Buffer, path: jtag::Path) -> Result<()>;

    async fn bytes(
        &mut self,
        buf: &mut dyn Buffer,
        before: Option<jtag::Path>,
        data: Data<'_>,
        after: Option<jtag::Path>,
    ) -> Result<()>;

    async fn bits(
        &mut self,
        buf: &mut dyn Buffer,
        before: Option<jtag::Path>,
        data: u32,
        len: Bits<u8>,
        after: Option<jtag::Path>,
    ) -> Result<()>;

    /// Run any queud IO commands
    async fn flush(&mut self, buf: &mut dyn Buffer) -> Result<()>;
}

pub trait Buffer: Send {
    /// Extend the buffer, returning a mutable slice to the newly-allocated
    /// memory
    fn extend(&mut self, size: usize) -> &mut [u8];

    /// Notify that `size` bytes were written. Used for progress bars.
    #[expect(unused)]
    fn notify_write(&mut self, size: usize) {}
}

#[async_trait::async_trait]
impl<B: Backend + ?Sized + Send> Backend for Box<B> {
    async fn tms(&mut self, buf: &mut dyn Buffer, path: jtag::Path) -> Result<()> {
        B::tms(self, buf, path).await
    }

    async fn bytes(
        &mut self,
        buf: &mut dyn Buffer,
        before: Option<jtag::Path>,
        data: Data<'_>,
        after: Option<jtag::Path>,
    ) -> Result<()> {
        B::bytes(self, buf, before, data, after).await
    }

    async fn bits(
        &mut self,
        buf: &mut dyn Buffer,
        before: Option<jtag::Path>,
        data: u32,
        len: Bits<u8>,
        after: Option<jtag::Path>,
    ) -> Result<()> {
        B::bits(self, buf, before, data, len, after).await
    }

    async fn flush(&mut self, buf: &mut dyn Buffer) -> Result<()> {
        B::flush(&mut *self, buf).await
    }
}

impl Buffer for Vec<u8> {
    fn extend(&mut self, size: usize) -> &mut [u8] {
        let len = self.len();
        self.resize(len + size, 0);
        &mut self[len..]
    }
}
