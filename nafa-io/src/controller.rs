use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use eyre::{Result, eyre};

use crate::{
    Backend, Buffer,
    backend::Data,
    devices::DeviceInfo,
    jtag::{self, PATHS, State},
    units::Bytes,
};

pub struct Controller<B> {
    backend: B,
    pub info: DeviceInfo,
    notify: Option<std::ptr::NonNull<AtomicUsize>>,
    buf: Vec<u8>,
}

impl<B: Backend> Controller<B> {
    pub fn new(mut backend: B, devices: &HashMap<u32, DeviceInfo>) -> Result<Self> {
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
        let id = u32::from_le_bytes(*id);
        let info = devices
            .get(&id)
            .ok_or_else(|| eyre!("idcode {id:08X} not found in device list"))?
            .clone();
        assert!(info.irlen.0 <= 32);

        buf.clear();
        Ok(Self {
            backend,
            buf,
            info,
            notify: None,
        })
    }

    pub fn info(&self) -> &DeviceInfo {
        &self.info
    }

    pub fn with_notifications<T>(
        &mut self,
        notify: &AtomicUsize,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let old_notify = self.notify.replace(std::ptr::NonNull::from_ref(notify));
        let r = f(self);
        self.notify = old_notify;
        r
    }

    /// Run a set of commands, returning the data read out of TDO.
    ///
    /// Before the first command is run, the JTAG will be in
    /// [`jtag::State::RunTestIdle`].
    ///
    /// When IO occurs, the number of bytes read is sent over `sender`.
    pub fn run<'d>(&mut self, commands: impl IntoIterator<Item = Command<'d>>) -> Result<&[u8]> {
        let Self {
            backend,
            buf,
            info,
            notify,
        } = self;
        let notify = notify.map(|n| unsafe { n.as_ref() });
        buf.clear();

        let ir0 = Some(PATHS[State::RunTestIdle][State::ShiftIR]);
        let ir1 = Some(PATHS[State::ShiftIR][State::RunTestIdle]);
        let dr0 = Some(PATHS[State::RunTestIdle][State::ShiftDR]);
        let dr1 = Some(PATHS[State::ShiftDR][State::RunTestIdle]);

        let mut last_noisy = false;
        for command in commands {
            last_noisy = command.notify;
            let buf: &mut dyn Buffer = match (notify, command.notify) {
                (Some(notify), true) => &mut NoisyBuffer { notify, buf },
                _ => buf,
            };
            match command.inner {
                CommandInner::IrTxBits { tdi } => backend.bits(buf, ir0, tdi, info.irlen, ir1)?,
                CommandInner::DrTx { tdi } => backend.bytes(buf, dr0, Data::Tx(tdi), dr1)?,
                CommandInner::DrRx { len } => backend.bytes(buf, dr0, Data::Rx(len), dr1)?,
                CommandInner::DrTxRx { tdi } => backend.bytes(buf, dr0, Data::TxRx(tdi), dr1)?,
                CommandInner::Idle { len } => {
                    backend.bytes(buf, None, Data::ConstantTx(false, len), None)?
                }
            }
        }

        let buf: &mut dyn Buffer = match (notify, last_noisy) {
            (Some(notify), true) => &mut NoisyBuffer { notify, buf },
            _ => buf,
        };

        backend.tms(buf, jtag::Path::IDLE)?;
        backend.flush(buf)?;
        Ok(&self.buf)
    }
}

struct NoisyBuffer<'d> {
    notify: &'d AtomicUsize,
    buf: &'d mut Vec<u8>,
}

impl Buffer for NoisyBuffer<'_> {
    fn extend(&mut self, size: usize) -> &mut [u8] {
        self.notify.fetch_add(size, Ordering::Relaxed);
        Buffer::extend(self.buf, size)
    }

    fn notify_write(&mut self, size: usize) {
        self.notify.fetch_add(size, Ordering::Relaxed);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Command<'d> {
    notify: bool,
    inner: CommandInner<'d>,
}

#[derive(Clone, Copy, Debug)]
enum CommandInner<'d> {
    IrTxBits { tdi: u32 },

    DrTx { tdi: &'d [u8] },
    DrRx { len: Bytes<usize> },
    DrTxRx { tdi: &'d [u8] },

    Idle { len: Bytes<usize> },
}

impl<'d> Command<'d> {
    pub fn ir(tdi: u32) -> Self {
        let inner = CommandInner::IrTxBits { tdi };
        let notify = false;
        Self { notify, inner }
    }

    pub fn dr_tx(tdi: &'d [u8]) -> Self {
        let inner = CommandInner::DrTx { tdi };
        let notify = false;
        Self { notify, inner }
    }

    pub fn dr_tx_with_notification(tdi: &'d [u8]) -> Self {
        let inner = CommandInner::DrTx { tdi };
        let notify = true;
        Self { notify, inner }
    }

    pub fn dr_rx(len: Bytes<usize>) -> Self {
        let inner = CommandInner::DrRx { len };
        let notify = false;
        Self { notify, inner }
    }

    pub fn dr_rx_with_notification(len: Bytes<usize>) -> Self {
        let inner = CommandInner::DrRx { len };
        let notify = true;
        Self { notify, inner }
    }

    pub fn dr_txrx(tdi: &'d [u8]) -> Self {
        let inner = CommandInner::DrTxRx { tdi };
        let notify = false;
        Self { notify, inner }
    }

    pub fn dr_txrx_with_notification(tdi: &'d [u8]) -> Self {
        let inner = CommandInner::DrTxRx { tdi };
        let notify = true;
        Self { notify, inner }
    }

    pub fn idle(len: Bytes<usize>) -> Self {
        let inner = CommandInner::Idle { len };
        let notify = false;
        Self { notify, inner }
    }
}
