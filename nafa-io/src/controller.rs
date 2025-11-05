use eyre::Result;

use crate::{
    Backend, Buffer, jtag,
    units::{Bits, Bytes},
};

pub struct Controller<B> {
    backend: B,
    buf: Vec<u8>,
}

impl<B: Backend> Controller<B> {
    pub fn new(mut backend: B) -> Result<Self> {
        let mut buf = Vec::new();
        backend.tms(&mut buf, jtag::Path::IDLE)?;
        backend.flush(&mut buf)?;
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

        // state must start in idle by contract
        let mut state = jtag::State::RunTestIdle;
        let mut it = commands.into_iter().peekable();
        while let Some(command) = it.next() {
            let last = it.peek().is_none_or(|c| matches!(c, Command::SetState(_)));
            match command {
                Command::SetState(new_state) => {
                    backend.tms(buf, jtag::PATHS[state][new_state])?;
                    state = new_state;
                }
                Command::TxBytes { tdi } => backend.tdi_bytes(buf, tdi, last)?,
                Command::TxBits { tdi, len } => backend.tdi_bits(buf, tdi, len, last)?,
                Command::RxBytes { len } => backend.tdo_bytes(buf, len, last)?,
                Command::TxRxBytes { tdi } => backend.tdi_tdo_bytes(buf, tdi, last)?,
            }
        }

        backend.tms(buf, jtag::Path::IDLE)?;
        backend.flush(buf)?;
        Ok(&self.buf)
    }
}

// TODO: this is maybe the wrong API for when there's more than one device on
// the JTAG chain.
//
// In that case, you need to start sending the data before you're even in the
// correct state, to shift through the bypass/instruction registers of whatever
// devices are before your target in the chain.
//
// The correct API maybe looks something like:
// ```rust,ignore
// enum Command {
//     Ir { command: u8 },
//     DrTx { data: &[u8] },
//     DrRx { len: Bytes<usize> },
//     DrTxRx { data: &[u8] },
//     Idle { len: Bytes<usize> },
// }
// ```
//
// That is, _only_ support sending bits to the instruction register and bytes to
// the data register. This covers all the current use cases. Given the JTAG
// state machine, it's not even _possible_ to stay in most of the states. You're
// only supposed to be transmitting data in the "0-stable" states --- ShiftIR,
// ShiftDR, PauseIR, PauseDR, and Idle.
//
// For all current devices, the Data Register expects some sort of byte-bounded
// communication. It doesn't really make sense to send, e.g., 5 bits to the
// CFG_IN of an FPGA. Similarly, the Instruction Register never expects more
// than 6 bits.
//
// Having the ability to send bytes while in idle is useful for cases like
// waiting for the XADC write to finish.
//
// Currenly no need to send data to the other 0-stable states (PauseIR,
// PauseDR).
//
// So, TLDR, options should be:
// - idle -> shiftir, send bits, shiftir -> idle
// - idle -> shiftdr, send bytes, shiftdr -> idle
// - send bytes (resting in idle)
#[derive(Clone, Copy, Debug)]
pub enum Command<'d> {
    SetState(jtag::State),

    TxBytes { tdi: &'d [u8] },
    TxBits { tdi: u8, len: Bits<usize> },

    RxBytes { len: Bytes<usize> },

    TxRxBytes { tdi: &'d [u8] },
}

impl<'d> Command<'d> {
    pub fn set_state(state: jtag::State) -> Self {
        Self::SetState(state)
    }

    pub fn tx_bytes(tdi: &'d [u8]) -> Self {
        Self::TxBytes { tdi }
    }
    pub fn tx_bits(tdi: u8, len: Bits<usize>) -> Self {
        Self::TxBits { tdi, len }
    }

    pub fn rx_bytes(len: Bytes<usize>) -> Self {
        Self::RxBytes { len }
    }

    pub fn tx_rx_bytes(tdi: &'d [u8]) -> Self {
        Self::TxRxBytes { tdi }
    }
}
