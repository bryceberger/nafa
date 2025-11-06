use eyre::{Result, eyre};

use crate::{
    Backend, Buffer,
    backend::Data,
    jtag::{self, PATHS, State},
    units::{Bits, Bytes},
};

pub struct Controller<B> {
    backend: B,
    buf: Vec<u8>,
}

impl<B: Backend> Controller<B> {
    pub fn new(mut backend: B) -> Result<Self> {
        let mut buf = Vec::new();
        backend.tms(&mut buf, jtag::Path::RESET)?;
        backend.tms(&mut buf, jtag::Path::RESET)?;
        let before = Some(jtag::PATHS[State::TestLogicReset][State::ShiftDR]);
        let after = Some(jtag::PATHS[State::ShiftDR][State::RunTestIdle]);
        backend.bytes(&mut buf, before, Data::Rx(Bytes(8)), after)?;
        backend.flush(&mut buf)?;

        let [id, extra] = buf.as_chunks().0 else {
            return Err(eyre!("backend failed to fill buffer"));
        };
        if u32::from_le_bytes(*extra) & 0xffff_ff00 != 0xffff_ff00 {
            return Err(eyre!("multiple devices detected on jtag chain"));
        }
        let _id = u32::from_le_bytes(*id);

        buf.clear();
        Ok(Self { backend, buf })
    }

    /// Run a set of commands, returning the data read out of TDO.
    ///
    /// Before the first command is run, the JTAG will be in
    /// [`jtag::State::RunTestIdle`].
    ///
    /// When IO occurs, the number of bytes read is sent over `sender`.
    pub fn run<'d>(&mut self, commands: impl IntoIterator<Item = Command<'d>>) -> Result<&[u8]> {
        let Self { backend, buf } = self;
        buf.clear();

        let buf: &mut dyn Buffer = buf;

        let ir0 = Some(PATHS[State::RunTestIdle][State::ShiftIR]);
        let ir1 = Some(PATHS[State::ShiftIR][State::RunTestIdle]);
        let dr0 = Some(PATHS[State::RunTestIdle][State::ShiftDR]);
        let dr1 = Some(PATHS[State::ShiftDR][State::RunTestIdle]);

        for command in commands {
            match command {
                Command::IrTxBits { tdi, len } => backend.bits(buf, ir0, tdi, len, ir1)?,
                Command::DrTx { tdi } => backend.bytes(buf, dr0, Data::Tx(tdi), dr1)?,
                Command::DrRx { len } => backend.bytes(buf, dr0, Data::Rx(len), dr1)?,
                Command::DrTxRx { tdi } => backend.bytes(buf, dr0, Data::TxRx(tdi), dr1)?,
                Command::Idle { len } => {
                    backend.bytes(buf, None, Data::ConstantTx(false, len), None)?
                }
            }
        }

        backend.tms(buf, jtag::Path::IDLE)?;
        backend.flush(buf)?;
        Ok(&self.buf)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Command<'d> {
    IrTxBits { tdi: u32, len: Bits<u8> },

    DrTx { tdi: &'d [u8] },
    DrRx { len: Bytes<usize> },
    DrTxRx { tdi: &'d [u8] },

    Idle { len: Bytes<usize> },
}

impl<'d> Command<'d> {
    pub fn ir(tdi: u32, len: Bits<u8>) -> Self {
        Self::IrTxBits { tdi, len }
    }

    pub fn dr_tx(tdi: &'d [u8]) -> Self {
        Self::DrTx { tdi }
    }

    pub fn dr_rx(len: Bytes<usize>) -> Self {
        Self::DrRx { len }
    }

    pub fn dr_txrx(tdi: &'d [u8]) -> Self {
        Self::DrTxRx { tdi }
    }

    pub fn idle(len: Bytes<usize>) -> Self {
        Self::Idle { len }
    }
}
