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

// `*const T` isn't `Send`, mostly as a lint. However, we want `Controller` to
// be `Send`.
//
// The only pointer that's used in it (`.notify`) has no complex lifetime
// requirements. It's only exposed by `.with_notifications()`, which takes in a
// reference and only uses the pointer while inside the function.
#[repr(transparent)]
struct Ptr<T>(*const T);
unsafe impl<T> Send for Ptr<T> where for<'a> &'a T: Send {}
unsafe impl<T> Sync for Ptr<T> where for<'a> &'a T: Sync {}
impl<T> Clone for Ptr<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Ptr<T> {}

pub struct Controller<B> {
    backend: B,
    idcode: u32,
    before: Vec<DeviceInfo>,
    info: DeviceInfo,
    after: Vec<DeviceInfo>,
    notify: Ptr<AtomicUsize>,
    buf: Vec<u8>,
}

#[tracing::instrument(skip_all)]
pub async fn detect_chain<B: Backend>(
    backend: &mut B,
    devices: &HashMap<IdCode, DeviceInfo>,
) -> Result<Vec<(u32, DeviceInfo)>> {
    let buf = &mut Vec::new();
    let reset_to_idle = PATHS[State::TestLogicReset][State::RunTestIdle];
    backend.tms(buf, Path::RESET).await?;
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
            idcode if ret.is_empty() && idcode & 0xfff == (0x093 << 1) => {
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

#[tracing::instrument(skip_all)]
async fn zynq_us_init_arm_dap<B: Backend>(backend: &mut B, buf: &mut Vec<u8>) -> Result<u32> {
    let reset_to_sir = Some(PATHS[State::TestLogicReset][State::ShiftIR]);
    let sir_to_idle = Some(PATHS[State::ShiftIR][State::RunTestIdle]);
    let idle_to_sdr = Some(PATHS[State::RunTestIdle][State::ShiftDR]);
    let sdr_to_reset = Some(PATHS[State::ShiftDR][State::TestLogicReset]);
    let reset_to_sdr = Some(PATHS[State::TestLogicReset][State::ShiftDR]);
    let sdr_to_idle = Some(PATHS[State::ShiftDR][State::RunTestIdle]);

    let jtag_ctrl = 0b100000 << (4 + 6) | 0b100100 << 4 | 0b1111;
    let enable_cmd = Data::Tx(&[0b0000_0011, 0x00, 0x00, 0x00]);
    let ones = Data::ConstantTx(true, Bytes(4));
    let rx_4 = Data::Rx(Bytes(4));

    backend.tms(buf, Path::RESET).await?;
    backend
        .bits(buf, reset_to_sir, jtag_ctrl, Bits(16), sir_to_idle)
        .await?;
    backend
        .bytes(buf, idle_to_sdr, enable_cmd, sdr_to_reset)
        .await?;
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
    #[tracing::instrument(skip_all)]
    pub async fn new(
        mut backend: B,
        before: Vec<DeviceInfo>,
        (idcode, info): (u32, DeviceInfo),
        after: Vec<DeviceInfo>,
    ) -> Result<Self> {
        let mut buf = Vec::new();
        let reset_to_idle = PATHS[State::TestLogicReset][State::RunTestIdle];
        backend.tms(&mut buf, Path::RESET).await?;
        backend.tms(&mut buf, reset_to_idle).await?;
        backend.flush(&mut buf).await?;
        buf.clear();

        Ok(Self {
            backend,
            buf,
            before,
            info,
            after,
            idcode,
            notify: Ptr(std::ptr::null()),
        })
    }

    pub fn info(&self) -> &DeviceInfo {
        &self.info
    }

    pub fn idcode(&self) -> u32 {
        self.idcode
    }

    pub async fn with_notifications<T>(
        &mut self,
        notify: &AtomicUsize,
        f: impl AsyncFnOnce(&mut Self) -> T,
    ) -> T {
        // TODO: this _technically_ exposes some unsafety. If the user-provided closure
        // panics, and the panic is later caught, we will be holding a
        // potentially-dangling pointer.
        //
        // This _should_ use [PPYP](https://faultlore.com/blah/everyone-poops/).
        let old_notify = std::mem::replace(&mut self.notify, Ptr(notify));
        let r = f(self).await;
        self.notify = old_notify;
        r
    }

    /// Run a set of commands, returning the data read out of TDO.
    ///
    /// Before the first command is run, the JTAG will be in
    /// [`State::RunTestIdle`].
    ///
    /// When IO occurs, the number of bytes read is sent over `sender`.
    #[tracing::instrument(skip_all)]
    pub async fn run<'d>(
        &mut self,
        commands: impl IntoIterator<Item = Command<'d>>,
    ) -> Result<&[u8]> {
        let Self {
            ref mut backend,
            ref mut buf,
            ref before,
            ref info,
            ref after,
            notify,
            idcode: _,
        } = *self;
        let notify = if notify.0.is_null() {
            None
        } else {
            Some(unsafe { &*notify.0 })
        };
        buf.clear();

        let irlen_before: Bits<u8> = Bits(before.iter().map(|i| i.irlen.0).sum());
        let irlen_after: Bits<u8> = Bits(after.iter().map(|i| i.irlen.0).sum());

        let devices_before = before.len();
        let devices_after = after.len();
        assert!(devices_before <= 32);
        assert!(devices_after <= 32);
        let devices_before = devices_before as u8;
        let devices_after = devices_after as u8;

        let mut last_noisy = false;
        for command in commands {
            last_noisy = command.notify;
            let buf: &mut dyn Buffer = match (notify, command.notify) {
                (Some(notify), true) => &mut NoisyBuffer { notify, buf },
                _ => buf,
            };
            match command.inner {
                CommandInner::IrTxBits { tdi } => {
                    io_bits(backend, buf, info, irlen_before, irlen_after, tdi).await?
                }
                CommandInner::DrTx { tdi } => {
                    io_bytes(backend, buf, devices_before, devices_after, Data::Tx(tdi)).await?
                }
                CommandInner::DrRx { len } => {
                    io_bytes(backend, buf, devices_before, devices_after, Data::Rx(len)).await?
                }
                CommandInner::DrTxRx { tdi } => {
                    io_bytes(backend, buf, devices_before, devices_after, Data::TxRx(tdi)).await?
                }
                CommandInner::Idle { len } => {
                    backend
                        .bytes(buf, None, Data::ConstantTx(true, len), None)
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

async fn io_bits<B: Backend>(
    backend: &mut B,
    buf: &mut dyn Buffer,
    info: &DeviceInfo,
    irlen_before: Bits<u8>,
    irlen_after: Bits<u8>,
    tdi: u32,
) -> Result<()> {
    let ir0 = Some(PATHS[State::RunTestIdle][State::ShiftIR]);
    let ir1 = Some(PATHS[State::ShiftIR][State::RunTestIdle]);

    match (irlen_before, irlen_after) {
        (Bits(0), Bits(0)) => {
            backend.bits(buf, ir0, tdi, info.irlen, ir1).await?;
        }
        (pre, Bits(0)) => {
            backend.bits(buf, ir0, u32::MAX, pre, None).await?;
            backend.bits(buf, None, tdi, info.irlen, ir1).await?;
        }
        (Bits(0), post) => {
            backend.bits(buf, ir0, tdi, info.irlen, None).await?;
            backend.bits(buf, None, u32::MAX, post, ir1).await?;
        }
        (pre, post) => {
            backend.bits(buf, ir0, u32::MAX, pre, None).await?;
            backend.bits(buf, None, tdi, info.irlen, None).await?;
            backend.bits(buf, None, u32::MAX, post, ir1).await?;
        }
    }
    Ok(())
}

async fn io_bytes<B: Backend>(
    backend: &mut B,
    buf: &mut dyn Buffer,
    devices_before: u8,
    devices_after: u8,
    data: Data<'_>,
) -> Result<()> {
    let dr0 = Some(PATHS[State::RunTestIdle][State::ShiftDR]);
    let dr1 = Some(PATHS[State::ShiftDR][State::RunTestIdle]);

    match (devices_before, devices_after) {
        (0, 0) => {
            backend.bytes(buf, dr0, data, dr1).await?;
        }
        (pre, 0) => {
            backend.bits(buf, dr0, u32::MAX, Bits(pre), None).await?;
            backend.bytes(buf, None, data, dr1).await?;
        }
        (0, post) => {
            backend.bytes(buf, dr0, data, None).await?;
            backend.bits(buf, None, u32::MAX, Bits(post), dr1).await?;
        }
        (pre, post) => {
            backend.bits(buf, dr0, u32::MAX, Bits(pre), None).await?;
            backend.bytes(buf, None, data, None).await?;
            backend.bits(buf, None, u32::MAX, Bits(post), dr1).await?;
        }
    }

    Ok(())
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
