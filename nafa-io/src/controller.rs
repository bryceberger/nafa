use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use eyre::{Result, eyre};

use crate::{
    Backend, Buffer,
    backend::Data,
    devices::{DeviceInfo, IdCode},
    jtag::{PATHS, Path, State},
    units::{Bits, Bytes},
};

pub struct Controller<B> {
    backend: B,
    pub idcode: u32,
    pub info: DeviceInfo,
    notify: Option<std::ptr::NonNull<AtomicUsize>>,
    buf: Vec<u8>,
}

async fn detect_chain<B: Backend>(
    backend: &mut B,
    devices: &HashMap<IdCode, DeviceInfo>,
) -> Result<Vec<(u32, DeviceInfo)>> {
    let buf = &mut Vec::new();
    let reset_to_idle = PATHS[State::TestLogicReset][State::RunTestIdle];
    backend.tms(buf, Path::RESET).await?;
    backend.tms(buf, reset_to_idle).await?;

    let idle_to_sdr = Some(PATHS[State::RunTestIdle][State::ShiftDR]);
    let sdr_to_idle = Some(PATHS[State::ShiftDR][State::RunTestIdle]);

    let get_info = |idcode| -> Result<DeviceInfo> {
        let Some(info) = devices.get(&IdCode::new(idcode)) else {
            return Err(eyre!("idcode {idcode:08X} not found in device list"));
        };
        assert!(info.irlen <= Bits(32));
        Ok(info.clone())
    };

    let mut ret = Vec::new();
    loop {
        let to_read = Bytes((ret.len() + 1) * 4);
        backend
            .bytes(buf, idle_to_sdr, Data::Rx(to_read), sdr_to_idle)
            .await?;
        backend.flush(buf).await?;

        let (ids, []) = buf.as_chunks() else {
            return Err(eyre!(
                "failed to fill idcode, or returned extra data: {buf:02X?}"
            ));
        };
        let id = ids[ret.len()];
        match u32::from_le_bytes(id) {
            // reached end of chain
            0xffff_ffff => {
                break;
            }

            // special case for Zynq US+: add ARM_DAP to the chain
            idcode if idcode & 0xfff == (0x093 << 1) && ret.is_empty() => {
                buf.clear();
                let idcode = zynq_us_init_arm_dap(backend, buf).await?;
                let info = get_info(idcode)?;
                ret.push((idcode, info.clone()));
            }

            // IDCODE guaranteed to start with a `1` bit, BYPASS as a single `0`. Nothing we can
            // really do with a device in BYPASS. Could scan a pattern through IR until we see our
            // input fed back, but that's annoying. All devices we care about start with IDCODE
            // anyway.
            idcode if idcode & 1 != 1 => {
                return Err(eyre!("device in BYPASS detected: {idcode:08X}",));
            }

            idcode => {
                let info = get_info(idcode)?;
                ret.push((idcode, info.clone()));
            }
        }
        buf.clear();
    }

    Ok(ret)
}

async fn zynq_us_init_arm_dap<B: Backend>(backend: &mut B, buf: &mut Vec<u8>) -> Result<u32> {
    let reset_to_sir = Some(PATHS[State::TestLogicReset][State::ShiftIR]);
    let sir_to_idle = Some(PATHS[State::ShiftIR][State::RunTestIdle]);
    let idle_to_sdr = Some(PATHS[State::RunTestIdle][State::ShiftDR]);
    let sdr_to_reset = Some(PATHS[State::ShiftDR][State::TestLogicReset]);
    let reset_to_sdr = Some(PATHS[State::TestLogicReset][State::ShiftDR]);
    let sdr_to_idle = Some(PATHS[State::ShiftDR][State::RunTestIdle]);

    let jtag_ctrl = 0b100000 << (4 + 6) | 0b100100 << 4 | 0b1111;
    let ones = Data::ConstantTx(true, Bytes(4));
    let rx_4 = Data::Rx(Bytes(4));

    backend.tms(buf, Path::RESET).await?;
    backend
        .bits(buf, reset_to_sir, jtag_ctrl, Bits(16), sir_to_idle)
        .await?;
    backend.bytes(buf, idle_to_sdr, ones, sdr_to_reset).await?;
    backend.bytes(buf, reset_to_sdr, ones, sdr_to_reset).await?;
    backend.bytes(buf, reset_to_sdr, rx_4, sdr_to_idle).await?;
    backend.flush(buf).await?;

    let ([id], []) = buf.as_chunks() else {
        return Err(eyre!("failed to get idcode after zynq us special case"));
    };
    match u32::from_le_bytes(*id) {
        0xffff_ffff => Err(eyre!("end of chain after zynq us special case ???")),
        idcode if idcode & 1 != 1 => Err(eyre!("still in bypass after zynq us special case")),
        idcode => Ok(idcode),
    }
}

impl<B: Backend> Controller<B> {
    pub async fn new(mut backend: B, devices: &HashMap<IdCode, DeviceInfo>) -> Result<Self> {
        let (idcode, info) = match &detect_chain(&mut backend, devices).await?[..] {
            [single] => single.clone(),
            [] => {
                return Err(eyre!("no devices detected on jtag chain"));
            }
            multiple => {
                let idcodes = multiple
                    .iter()
                    .map(|(idcode, _)| idcode)
                    .collect::<Vec<_>>();
                return Err(eyre!(
                    "multiple devices detected on jtag chain: {idcodes:08X?}"
                ));
            }
        };
        Ok(Self {
            backend,
            buf: Vec::new(),
            info,
            idcode,
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
    /// [`State::RunTestIdle`].
    ///
    /// When IO occurs, the number of bytes read is sent over `sender`.
    pub async fn run<'d>(
        &mut self,
        commands: impl IntoIterator<Item = Command<'d>>,
    ) -> Result<&[u8]> {
        let Self {
            backend,
            buf,
            info,
            notify,
            idcode: _,
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
                CommandInner::IrTxBits { tdi } => {
                    backend.bits(buf, ir0, tdi, info.irlen, ir1).await?
                }
                CommandInner::DrTx { tdi } => backend.bytes(buf, dr0, Data::Tx(tdi), dr1).await?,
                CommandInner::DrRx { len } => backend.bytes(buf, dr0, Data::Rx(len), dr1).await?,
                CommandInner::DrTxRx { tdi } => {
                    backend.bytes(buf, dr0, Data::TxRx(tdi), dr1).await?
                }
                CommandInner::Idle { len } => {
                    backend
                        .bytes(buf, None, Data::ConstantTx(false, len), None)
                        .await?
                }
            }
        }

        let buf: &mut dyn Buffer = match (notify, last_noisy) {
            (Some(notify), true) => &mut NoisyBuffer { notify, buf },
            _ => buf,
        };

        backend.tms(buf, Path::IDLE).await?;
        backend.flush(buf).await?;
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
