use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use color_eyre::{Section as _, SectionExt as _};
use eyre::{Result, eyre};

use crate::{
    Backend, Buffer,
    backend::Data,
    devices::DeviceInfo,
    jtag::{IdCode, PATHS, Path, State},
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
    before: Vec<(IdCode, DeviceInfo)>,
    active: (IdCode, DeviceInfo),
    after: Vec<(IdCode, DeviceInfo)>,
    notify: Ptr<AtomicUsize>,
    buf: Vec<u8>,
}

fn max_possible_combined_irlen(ircapture: &[u8]) -> Bits<usize> {
    let Some(pos) = ircapture.iter().rposition(|x| *x != 0xff) else {
        return Bits(0);
    };
    let mut val = ircapture[pos];
    let mut extra = 0;
    while val & 0x80 == 0x80 {
        val <<= 1;
        extra += 1;
    }
    Bits(pos * 8 + 8 - extra)
}

pub struct IdCodeInfo<'a> {
    pub indent: usize,
    pub idcode: IdCode,
    pub info: Option<&'a DeviceInfo>,
}
impl<'a> IdCodeInfo<'a> {
    pub fn new(indent: usize, idcode: IdCode, info: Option<&'a DeviceInfo>) -> Self {
        Self {
            indent,
            idcode,
            info,
        }
    }
}
impl std::fmt::Display for IdCodeInfo<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            concat!(
                "{i:il$}manufacturer 0x{mfg:03X}  ({mfg_str})\n",
                "{i:il$}part         0x{part:04X} ({part_str})\n",
                "{i:il$}version      0x{version:X}\n",
                "{i:il$}irlen        {irlen}",
            ),
            i = "",
            il = self.indent,
            mfg = self.idcode.manufacturer(),
            mfg_str = self.idcode.manufacturer_name().unwrap_or("<unknown>"),
            part = self.idcode.part(),
            part_str = self.info.map_or("<unknown>", |p| p.name),
            version = self.idcode.version(),
            irlen = self.info.map_or(0, |i| i.irlen.0),
        )
    }
}

fn get_info(
    devices: &HashMap<IdCode, DeviceInfo>,
    chain: &[(IdCode, DeviceInfo)],
    idcode: IdCode,
) -> Result<DeviceInfo> {
    let info = devices
        .get(&idcode)
        .or_else(|| devices.get(&idcode.strip_version()));
    let Some(info) = info else {
        let mut err = eyre!(
            "idcode {code:08X} not found in device list\n{info}",
            code = idcode.code(),
            info = IdCodeInfo::new(4, idcode, None),
        )
        .wrap_err("cannot determine irlen");

        let shifted = IdCode::new(idcode.code() >> 1);
        if let info @ Some(_) = devices.get(&shifted) {
            let code = shifted.code();
            let info = IdCodeInfo::new(4, shifted, info);
            err = err.note(format!(
                "idcode {code:08X} was found in device list, possible device in bypass\n{info}"
            ));
        }

        err = err.section({
            use std::fmt::Write;
            let mut msg = String::new();
            for (idx, (idcode, info)) in chain.iter().enumerate() {
                let code = idcode.code();
                let info = IdCodeInfo::new(7, *idcode, Some(info));
                if idx != 0 {
                    msg.push('\n');
                }
                write!(&mut msg, "{idx}: idcode {code:08X}\n{info}")
                    .expect("write!() to string cannot fail");
            }
            msg.header("Current chain:")
        });

        return Err(err);
    };
    assert!(info.irlen <= Bits(32));
    Ok(info.clone())
}

#[tracing::instrument(skip_all)]
pub async fn detect_chain<B: Backend>(
    backend: &mut B,
    devices: &HashMap<IdCode, DeviceInfo>,
) -> Result<Vec<(IdCode, DeviceInfo)>> {
    let buf = &mut Vec::new();

    let to_sir = Some(PATHS[State::TestLogicReset][State::ShiftIR]);
    let to_reset = Some(PATHS[State::ShiftIR][State::TestLogicReset]);
    let capture = Data::TxRx(&[0xff; 16]);
    backend.tms(buf, Path::RESET).await?;
    backend.bytes(buf, to_sir, capture, to_reset).await?;
    backend.flush(buf).await?;
    tracing::info!(capture = %crate::ShortHex(buf), maybe_irlen = ?max_possible_combined_irlen(buf));
    buf.clear();

    let reset_to_idle = PATHS[State::TestLogicReset][State::RunTestIdle];
    backend.tms(buf, Path::RESET).await?;
    backend.tms(buf, Path::RESET).await?;
    backend.tms(buf, reset_to_idle).await?;

    let idle_to_sdr = Some(PATHS[State::RunTestIdle][State::ShiftDR]);
    let sdr_to_idle = Some(PATHS[State::ShiftDR][State::RunTestIdle]);

    let mut ret = Vec::new();
    let mut must_be_last = false;
    loop {
        let rx = Data::Rx(Bytes((ret.len() + 1) * 4));
        backend.bytes(buf, idle_to_sdr, rx, sdr_to_idle).await?;
        backend.flush(buf).await?;

        let (ids, []) = buf.as_chunks() else {
            return Err(eyre!(
                "failed to fill idcode, or returned extra data: {}",
                crate::ShortHex(buf),
            ));
        };
        let id = ids[ret.len()];
        tracing::info!(id = %crate::ShortHex(&id));
        match u32::from_le_bytes(id) {
            // reached end of chain
            0xffff_ffff => {
                break;
            }

            _idcode if must_be_last => {
                return Err(eyre!("device after intel 1-bit-tap special case"));
            }

            // special case for Zynq US+: add ARM_DAP to the chain
            idcode if ret.is_empty() && idcode & 0xfff == (0x093 << 1) => {
                buf.clear();
                let idcode = zynq_us_init_arm_dap(backend, buf).await?;
                let info = get_info(devices, &ret, idcode)?;
                ret.push((idcode, info));
            }

            // IDCODE guaranteed to start with a `1` bit, BYPASS as a single `0`. Nothing we can
            // really do with a device in BYPASS. Could scan a pattern through IR until we see our
            // input fed back, but that's annoying. All devices we care about start with IDCODE
            // anyway.
            idcode if idcode & 1 != 1 => {
                return Err(eyre!("device in BYPASS detected: {idcode:08X}"));
            }

            idcode => {
                let idcode = IdCode::new(idcode);
                if let Some((extra, info)) = intel_special_case(devices, idcode) {
                    ret.push(extra);
                    ret.push(info);
                    must_be_last = true;
                } else {
                    let info = get_info(devices, &ret, idcode)?;
                    ret.push((idcode, info));
                }
            }
        }
        buf.clear();
    }

    Ok(ret)
}

#[tracing::instrument(skip_all)]
async fn zynq_us_init_arm_dap<B: Backend>(backend: &mut B, buf: &mut Vec<u8>) -> Result<IdCode> {
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
        idcode => Ok(IdCode::new(idcode)),
    }
}

fn intel_special_case(
    devices: &HashMap<IdCode, DeviceInfo>,
    idcode: IdCode,
) -> Option<((IdCode, DeviceInfo), (IdCode, DeviceInfo))> {
    use crate::devices::Specific as S;
    let shifted = IdCode::new(idcode.code() >> 1);
    let info = devices.get(&shifted)?;
    if let S::Intel = info.specific {
        let fake_tap = DeviceInfo {
            irlen: Bits(1),
            name: "1_BIT_TAP",
            specific: S::Unknown,
        };
        Some(((IdCode::new(0x00000001), fake_tap), (shifted, info.clone())))
    } else {
        None
    }
}

impl<B: Backend> Controller<B> {
    #[tracing::instrument(skip_all)]
    pub async fn new(
        mut backend: B,
        before: Vec<(IdCode, DeviceInfo)>,
        active: (IdCode, DeviceInfo),
        after: Vec<(IdCode, DeviceInfo)>,
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
            active,
            after,
            notify: Ptr(std::ptr::null()),
        })
    }

    pub fn info(&self) -> &DeviceInfo {
        &self.active.1
    }

    pub fn idcode(&self) -> IdCode {
        self.active.0
    }

    pub fn info_before(&self) -> &[(IdCode, DeviceInfo)] {
        &self.before
    }

    pub fn info_after(&self) -> &[(IdCode, DeviceInfo)] {
        &self.after
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
            active: (_, ref info),
            ref after,
            notify,
        } = *self;
        let notify = if notify.0.is_null() {
            None
        } else {
            Some(unsafe { &*notify.0 })
        };
        buf.clear();

        let irlen_before: Bits<u8> = Bits(before.iter().map(|i| i.1.irlen.0).sum());
        let irlen_after: Bits<u8> = Bits(after.iter().map(|i| i.1.irlen.0).sum());
        let irlen = ChainInfo {
            before: irlen_before,
            after: irlen_after,
        };

        let devices_before = before.len();
        let devices_after = after.len();
        assert!(devices_before <= 32);
        assert!(devices_after <= 32);
        let devices = ChainInfo {
            before: devices_before as _,
            after: devices_after as _,
        };

        let mut last_noisy = false;
        for command in commands {
            last_noisy = command.notify;
            let buf: &mut dyn Buffer = match (notify, command.notify) {
                (Some(notify), true) => &mut NoisyBuffer { notify, buf },
                _ => buf,
            };
            match command.inner {
                CommandInner::IrTxBits { tdi } => {
                    let data = BitTx {
                        tdi,
                        len: info.irlen,
                    };
                    io_bits_ir(backend, buf, irlen, data).await?
                }
                CommandInner::DrTx { tdi } => {
                    io_bytes(backend, buf, devices, Data::Tx(tdi)).await?
                }
                CommandInner::DrRx { len } => {
                    io_bytes(backend, buf, devices, Data::Rx(len)).await?
                }
                CommandInner::DrTxRx { tdi } => {
                    io_bytes(backend, buf, devices, Data::TxRx(tdi)).await?
                }
                CommandInner::DrTxBits { tdi, len } => {
                    io_bits_dr(backend, buf, devices, BitTx { tdi, len }).await?;
                }
                CommandInner::CombinedIrDrTxBits { ir, dr, dr_len } => {
                    let ir = BitTx {
                        tdi: ir,
                        len: info.irlen,
                    };
                    let dr = BitTx {
                        tdi: dr,
                        len: dr_len,
                    };
                    io_bits_ir_dr(backend, buf, irlen, devices, ir, dr).await?
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

        backend.flush(buf).await?;
        Ok(&self.buf)
    }
}

#[derive(Clone, Copy)]
struct ChainInfo<T> {
    before: T,
    after: T,
}

#[derive(Clone, Copy)]
struct BitTx {
    tdi: u32,
    len: Bits<u8>,
}

async fn io_bits_ir<B: Backend>(
    backend: &mut B,
    buf: &mut dyn Buffer,
    irlen: ChainInfo<Bits<u8>>,
    ir: BitTx,
) -> Result<()> {
    let ir0 = Some(PATHS[State::RunTestIdle][State::ShiftIR]);
    let ir1 = Some(PATHS[State::ShiftIR][State::RunTestIdle]);

    match (irlen.before, irlen.after) {
        (Bits(0), Bits(0)) => {
            backend.bits(buf, ir0, ir.tdi, ir.len, ir1).await?;
        }
        (pre, Bits(0)) => {
            backend.bits(buf, ir0, u32::MAX, pre, None).await?;
            backend.bits(buf, None, ir.tdi, ir.len, ir1).await?;
        }
        (Bits(0), post) => {
            backend.bits(buf, ir0, ir.tdi, ir.len, None).await?;
            backend.bits(buf, None, u32::MAX, post, ir1).await?;
        }
        (pre, post) => {
            backend.bits(buf, ir0, u32::MAX, pre, None).await?;
            backend.bits(buf, None, ir.tdi, ir.len, None).await?;
            backend.bits(buf, None, u32::MAX, post, ir1).await?;
        }
    }
    Ok(())
}

async fn io_bits_dr(
    backend: &mut impl Backend,
    buf: &mut dyn Buffer,
    devices: ChainInfo<u8>,
    dr: BitTx,
) -> Result<()> {
    let dr0 = Some(PATHS[State::RunTestIdle][State::ShiftDR]);
    let dr1 = Some(PATHS[State::ShiftDR][State::RunTestIdle]);

    match (devices.before, devices.after) {
        (0, 0) => {
            backend.bits(buf, dr0, dr.tdi, dr.len, dr1).await?;
        }
        (pre, 0) => {
            backend.bits(buf, dr0, u32::MAX, Bits(pre), None).await?;
            backend.bits(buf, None, dr.tdi, dr.len, dr1).await?;
        }
        (0, post) => {
            backend.bits(buf, dr0, dr.tdi, dr.len, None).await?;
            backend.bits(buf, None, u32::MAX, Bits(post), dr1).await?;
        }
        (pre, post) => {
            backend.bits(buf, dr0, u32::MAX, Bits(pre), None).await?;
            backend.bits(buf, None, dr.tdi, dr.len, None).await?;
            backend.bits(buf, None, u32::MAX, Bits(post), dr1).await?;
        }
    }
    Ok(())
}

async fn io_bits_ir_dr(
    backend: &mut impl Backend,
    buf: &mut dyn Buffer,
    irlen: ChainInfo<Bits<u8>>,
    devices: ChainInfo<u8>,
    ir: BitTx,
    dr: BitTx,
) -> Result<()> {
    let ir0 = Some(PATHS[State::RunTestIdle][State::ShiftIR]);
    let ir1 = Some(PATHS[State::ShiftIR][State::PauseIR]);
    let dr0 = Some(PATHS[State::PauseIR][State::ShiftDR]);
    let dr1 = Some(PATHS[State::ShiftDR][State::RunTestIdle]);

    match (irlen.before, irlen.after) {
        (Bits(0), Bits(0)) => {
            backend.bits(buf, ir0, ir.tdi, ir.len, ir1).await?;
        }
        (pre, Bits(0)) => {
            backend.bits(buf, ir0, u32::MAX, pre, None).await?;
            backend.bits(buf, None, ir.tdi, ir.len, ir1).await?;
        }
        (Bits(0), post) => {
            backend.bits(buf, ir0, ir.tdi, ir.len, None).await?;
            backend.bits(buf, None, u32::MAX, post, ir1).await?;
        }
        (pre, post) => {
            backend.bits(buf, ir0, u32::MAX, pre, None).await?;
            backend.bits(buf, None, ir.tdi, ir.len, None).await?;
            backend.bits(buf, None, u32::MAX, post, ir1).await?;
        }
    }
    match (devices.before, devices.after) {
        (0, 0) => {
            backend.bits(buf, dr0, dr.tdi, dr.len, dr1).await?;
        }
        (pre, 0) => {
            backend.bits(buf, dr0, u32::MAX, Bits(pre), None).await?;
            backend.bits(buf, None, dr.tdi, dr.len, dr1).await?;
        }
        (0, post) => {
            backend.bits(buf, dr0, dr.tdi, dr.len, None).await?;
            backend.bits(buf, None, u32::MAX, Bits(post), dr1).await?;
        }
        (pre, post) => {
            backend.bits(buf, dr0, u32::MAX, Bits(pre), None).await?;
            backend.bits(buf, None, dr.tdi, dr.len, None).await?;
            backend.bits(buf, None, u32::MAX, Bits(post), dr1).await?;
        }
    }
    Ok(())
}

async fn io_bytes<B: Backend>(
    backend: &mut B,
    buf: &mut dyn Buffer,
    devices: ChainInfo<u8>,
    data: Data<'_>,
) -> Result<()> {
    let dr0 = Some(PATHS[State::RunTestIdle][State::ShiftDR]);
    let dr1 = Some(PATHS[State::ShiftDR][State::RunTestIdle]);

    match (devices.before, devices.after) {
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
    DrTxBits { tdi: u32, len: Bits<u8> },

    CombinedIrDrTxBits { ir: u32, dr: u32, dr_len: Bits<u8> },

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

    pub fn dr_tx_bits(tdi: u32, len: Bits<u8>) -> Self {
        let inner = CommandInner::DrTxBits { tdi, len };
        let notify = false;
        Self { notify, inner }
    }

    pub fn combined_ir_dr_tx_bits(ir: u32, dr: u32, dr_len: Bits<u8>) -> Self {
        let inner = CommandInner::CombinedIrDrTxBits { ir, dr, dr_len };
        let notify = false;
        Self { notify, inner }
    }

    pub fn idle(len: Bytes<usize>) -> Self {
        let inner = CommandInner::Idle { len };
        let notify = false;
        Self { notify, inner }
    }
}
