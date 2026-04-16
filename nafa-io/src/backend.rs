use eyre::Result;

use crate::{
    jtag,
    units::{Bits, Bytes},
};

#[derive(Clone, Copy)]
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

    // TODO: add a `capture_ir_and_flush()` function.
    //
    // I don't want to deal with bit-wise IO in the general case, with potentially
    // interleaved bit / byte IO.
    //
    // However, some devices (xilinx) encode useful status information in the
    // CAPTURE-IR value, that's not possible / annoying to get elsewhere.
    //
    // Therefore, add a function that:
    // - is assumed (and can assert) that it's called in isolation:
    //   - no outstanding reads / writes
    // - goes into shift ir, rxtx a specified number of `1`s
    //
    // Notably, it would be a "synchronous" function, in that you don't call
    // `.flush()` after and it always returns the data immediately.
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
    /// memory.
    ///
    /// The returned buffer will be of length `size + scratch`. `size` is the
    /// final data length, while `scratch` is extra space required for reading.
    fn extend(&mut self, size: usize, scratch: usize) -> &mut [u8];

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

pub struct ScratchBuffer {
    data: Vec<u8>,
    scratch: usize,
}

impl ScratchBuffer {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            scratch: 0,
        }
    }

    pub fn data(&self) -> &[u8] {
        let len = self.data.len() - self.scratch;
        &self.data[..len]
    }

    pub fn data_mut(&mut self) -> &mut [u8] {
        let len = self.data.len() - self.scratch;
        &mut self.data[..len]
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.scratch = 0;
    }
}

impl Default for ScratchBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl Buffer for ScratchBuffer {
    fn extend(&mut self, size: usize, scratch: usize) -> &mut [u8] {
        let len = self.data.len() - self.scratch;
        self.data.resize(len + size + scratch, 0);
        self.scratch = scratch;
        &mut self.data[len..]
    }
}
