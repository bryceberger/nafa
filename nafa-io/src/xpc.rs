use std::time::Duration;

use eyre::Result;
use rusb::{
    DeviceHandle, GlobalContext,
    constants::{LIBUSB_ENDPOINT_OUT, LIBUSB_REQUEST_TYPE_VENDOR},
};
use tracing::{debug, info, instrument};

use crate::{
    Backend, Buffer, Hex, SpaceHex,
    units::{Bits, Bytes},
};

pub mod firmware;

pub struct Device {
    handle: DeviceHandle<GlobalContext>,
    /// Data to transfer. 4 bits over the wire for every 2 bytes in buffer, with
    /// the following format:
    ///
    /// ```text
    /// buf[0]: ssss iiii
    /// buf[1]: oooo 1111
    /// buf[2]: ssss iiii
    /// buf[3]: oooo 1111
    /// ```
    ///
    /// - `s` -> TMS
    /// - `i` -> TDI
    /// - `o` -> sample TDO
    /// - `1` -> unknown, xc3sprog always sets to 1. Maybe TCK?
    ///
    /// Data is sent LSB-first. Given: `buf[0] = 0x9a`, `buf[1] = 0x3f`
    /// ```text
    /// ┌Signals──┐┌Waves────────────────────────────────┐
    /// │TCK      ││──┐   ┌───┐   ┌───┐   ┌───┐   ┌───┐  │
    /// │         ││  └───┘   └───┘   └───┘   └───┘   └──│
    /// │TMS(1001)││  ────────┐               ┌────────  │
    /// │         ││          └───────────────┘          │
    /// │TDI(0101)││          ┌───────┐       ┌────────  │
    /// │         ││  ────────┘       └───────┘          │
    /// │TDO(1100)││      │───s───│───s───│───────│──────│
    /// └─────────┘└─────────────────────────────────────┘
    /// ```
    cmd_buf: Vec<u8>,
    cmd_read_len: usize,
    num_bits: u8,
    last_tdi: Option<bool>,
    last_tdo: Option<bool>,
}

const XPCU_CTRL_LOAD_FIRM: u8 = 0xA0;
const EZUSB_CPUCS: u16 = 0xe600;
const CPU_RESET: u8 = 1;

const XPC_PROG: u16 = 1 << 3;
// const XPC_TCK: u16 = 1 << 2;
// const XPC_TMS: u16 = 1 << 1;
// const XPC_TDI: u16 = 1 << 0;
// const XPC_TDO: u16 = 1 << 0;

const S: Duration = Duration::from_secs(1);

pub fn flash<Ctx: rusb::UsbContext>(
    dev: &DeviceHandle<Ctx>,
    firmware: &[(u16, &[u8])],
) -> Result<()> {
    // A host loader program must write 0x01 to the CPUCS register
    // to put the CPU into RESET, load all or part of the EZUSB
    // RAM with firmware, then reload the CPUCS register
    // with ‘0’ to take the CPU out of RESET. The CPUCS register
    // (at 0xE600) is the only EZ-USB register that can be written
    // using the Firmware Download command.
    dev.claim_interface(0)?;
    let request_type = LIBUSB_REQUEST_TYPE_VENDOR | LIBUSB_ENDPOINT_OUT;
    let request = XPCU_CTRL_LOAD_FIRM;
    let index = 0;
    dev.write_control(request_type, request, EZUSB_CPUCS, index, &[CPU_RESET], S)?;

    for (addr, data) in firmware {
        let mut addr = *addr;
        for chunk in data.chunks(64) {
            dev.write_control(request_type, request, addr, index, chunk, S)?;
            addr += u16::try_from(chunk.len()).unwrap();
        }
    }

    dev.write_control(request_type, request, EZUSB_CPUCS, index, &[0], S)?;
    dev.release_interface(0)?;

    Ok(())
}

impl Device {
    pub fn new(h: DeviceHandle<GlobalContext>) -> Result<Self> {
        request_28(&h, 0x11)?;
        write_gpio(&h, XPC_PROG)?;
        info!(
            firmware_version = %Hex(read_firmware_version(&h)?),
            cpld_version = %Hex(read_cpld_version(&h)?),
        );

        request_28(&h, 0x11)?;
        output_enable(&h, true)?;
        shift(&h, 0xa6, 2, &[0x00; 2], None)?;
        request_28(&h, 0x12)?;

        Ok(Self {
            handle: h,
            cmd_buf: Vec::new(),
            cmd_read_len: 0,
            num_bits: 0,
            last_tdi: None,
            last_tdo: None,
        })
    }

    fn add_bit(&mut self, tms: bool, tdi: bool, tdo: bool) {
        self.add_bit_internal(tms, tdi, tdo, true);
    }

    fn add_bit_internal(&mut self, tms: bool, tdi: bool, tdo: bool, tck: bool) {
        if self.num_bits == 0 {
            self.cmd_buf.push(0);
            self.cmd_buf.push(0);
        }

        let len = self.cmd_buf.len();
        let [b0, b1] = &mut self.cmd_buf[len - 2..] else {
            unreachable!()
        };

        *b0 |= (tms as u8) << (self.num_bits + 4) | (tdi as u8) << self.num_bits;
        *b1 |= (tdo as u8) << (self.num_bits + 4) | (tck as u8) << self.num_bits;

        self.num_bits = (self.num_bits + 1) & 3;
    }

    fn maybe_flush(&mut self, buf: &mut dyn Buffer) -> Result<()> {
        const MAX_BUF_LEN: usize = 8192;
        if self.cmd_buf.len() >= MAX_BUF_LEN {
            self.flush(buf)?;
        }
        Ok(())
    }
}

fn request_28(h: &DeviceHandle<GlobalContext>, value: u16) -> Result<()> {
    let written = h.write_control(0x40, 0xb0, 0x0028, value, &[], S)?;
    assert_eq!(written, 0);
    Ok(())
}

fn write_gpio(h: &DeviceHandle<GlobalContext>, bits: u16) -> Result<()> {
    let written = h.write_control(0x40, 0xb0, 0x0030, bits, &[], S)?;
    assert_eq!(written, 0);
    Ok(())
}

fn read_firmware_version(h: &DeviceHandle<GlobalContext>) -> Result<u16> {
    let mut out = [0x00; size_of::<u16>()];
    let buf = out.as_mut_slice();
    let read = h.read_control(0xc0, 0xb0, 0x0050, 0x0000, buf, S)?;
    assert_eq!(read, size_of::<u16>());
    Ok(u16::from_le_bytes(out))
}

fn read_cpld_version(h: &DeviceHandle<GlobalContext>) -> Result<u16> {
    let mut out = [0x00; size_of::<u16>()];
    let buf = out.as_mut_slice();
    let read = h.read_control(0xc0, 0xb0, 0x0050, 0x0001, buf, S)?;
    assert_eq!(read, size_of::<u16>());
    Ok(u16::from_le_bytes(out))
}

fn output_enable(h: &DeviceHandle<GlobalContext>, enable: bool) -> Result<()> {
    let read = h.write_control(0x40, 0xb0, if enable { 0x18 } else { 0x10 }, 0, &[], S)?;
    assert_eq!(read, 0);
    Ok(())
}

fn shift(
    h: &DeviceHandle<GlobalContext>,
    reqno: u16,
    bits: u16,
    in_buf: &[u8],
    out_buf: Option<&mut [u8]>,
) -> Result<()> {
    let written = h.write_control(0x40, 0xb0, reqno, bits, &[], S)?;
    assert_eq!(written, 0);

    let written = h.write_bulk(0x02, in_buf, S)?;
    assert_eq!(written, in_buf.len());

    if let Some(out) = out_buf {
        let read = h.read_bulk(0x86, out, S)?;
        assert_eq!(read, out.len());
    }

    Ok(())
}

#[allow(unused)]
impl Backend for Device {
    #[instrument(skip_all)]
    fn tms(&mut self, buf: &mut dyn Buffer, path: crate::jtag::Path) -> Result<()> {
        let last_tdi = self.last_tdi.take().unwrap_or(false);
        let last_tdo = self.last_tdo.take().unwrap_or(false);

        for (idx, tms) in path.into_iter().enumerate() {
            if idx == 0 {
                self.add_bit(tms, last_tdi, last_tdo);
            } else {
                self.add_bit(tms, true, false);
            }
        }

        self.maybe_flush(buf);
        Ok(())
    }

    #[instrument(skip_all)]
    fn tdi_bytes(&mut self, buf: &mut dyn Buffer, tdi: &[u8], last: bool) -> Result<()> {
        assert!(self.last_tdi.is_none());
        assert!(self.last_tdo.is_none());

        let tms = false;
        let tdo = false;
        for (idx, byte) in tdi.iter().copied().enumerate() {
            self.add_bit(tms, byte & 1 != 0, tdo);
            self.add_bit(tms, byte >> 1 & 1 != 0, tdo);
            self.add_bit(tms, byte >> 2 & 1 != 0, tdo);
            self.add_bit(tms, byte >> 3 & 1 != 0, tdo);
            self.add_bit(tms, byte >> 4 & 1 != 0, tdo);
            self.add_bit(tms, byte >> 5 & 1 != 0, tdo);
            self.add_bit(tms, byte >> 6 & 1 != 0, tdo);
            if last && idx == tdi.len() - 1 {
                self.last_tdi = Some(byte >> 7 & 1 != 0);
            } else {
                self.add_bit(tms, byte >> 7 & 1 != 0, tdo);
            }

            self.maybe_flush(buf)?;
        }
        Ok(())
    }

    #[instrument(skip_all)]
    fn tdi_bits(
        &mut self,
        buf: &mut dyn Buffer,
        tdi: u8,
        len: Bits<usize>,
        last: bool,
    ) -> Result<()> {
        assert!(self.last_tdi.is_none());
        assert!(self.last_tdo.is_none());

        let (len, last) = if last {
            (len.0 - 1, Some(tdi >> (len.0 - 1) & 1 != 0))
        } else {
            (len.0, None)
        };

        let tms = false;
        let tdo = false;
        for idx in 0..len {
            self.add_bit(tms, tdi >> idx & 1 != 0, tdo);
        }

        self.last_tdi = last;

        self.maybe_flush(buf)
    }

    #[instrument(skip_all)]
    fn tdo_bytes(&mut self, buf: &mut dyn Buffer, len: Bytes<usize>, last: bool) -> Result<()> {
        assert!(self.last_tdi.is_none());
        assert!(self.last_tdo.is_none());
        let tms = false;
        let tdi = false;
        let tdo = true;
        for idx in 0..len.0 {
            self.add_bit(tms, tdi, tdo);
            self.add_bit(tms, tdi, tdo);
            self.add_bit(tms, tdi, tdo);
            self.add_bit(tms, tdi, tdo);
            self.add_bit(tms, tdi, tdo);
            self.add_bit(tms, tdi, tdo);
            self.add_bit(tms, tdi, tdo);
            if last && idx == len.0 - 1 {
                self.last_tdo = Some(tdo);
            } else {
                self.add_bit(tms, tdi, tdo);
            }

            self.cmd_read_len += 1;
            self.maybe_flush(buf)?;
        }
        Ok(())
    }

    #[instrument(skip_all)]
    fn tdi_tdo_bytes(&mut self, buf: &mut dyn Buffer, tdi: &[u8], last: bool) -> Result<()> {
        assert!(self.last_tdi.is_none());
        assert!(self.last_tdo.is_none());
        let tms = false;
        let tdo = true;
        for (idx, byte) in tdi.iter().copied().enumerate() {
            self.add_bit(tms, byte & 1 != 0, tdo);
            self.add_bit(tms, byte >> 1 & 1 != 0, tdo);
            self.add_bit(tms, byte >> 2 & 1 != 0, tdo);
            self.add_bit(tms, byte >> 3 & 1 != 0, tdo);
            self.add_bit(tms, byte >> 4 & 1 != 0, tdo);
            self.add_bit(tms, byte >> 5 & 1 != 0, tdo);
            self.add_bit(tms, byte >> 6 & 1 != 0, tdo);
            if idx == tdi.len() - 1 && last {
                self.last_tdi = Some(byte >> 7 & 1 != 0);
                self.last_tdo = Some(true);
            } else {
                self.add_bit(tms, byte >> 7 & 1 != 0, tdo);
            }

            self.cmd_read_len += 1;
            self.maybe_flush(buf)?;
        }
        Ok(())
    }

    #[instrument(skip_all)]
    fn flush(&mut self, buf: &mut dyn Buffer) -> Result<()> {
        if self.num_bits == 0 {
            self.add_bit_internal(false, false, false, false);
        }

        let buf = match self.cmd_read_len {
            0 => None,
            _ => Some(buf.extend(self.cmd_read_len)),
        };
        let extra_bits = if self.num_bits == 0 {
            4
        } else {
            usize::from(self.num_bits)
        };
        let in_bits = ((self.cmd_buf.len() - 2) / 2 * 4 + extra_bits);
        let in_bits = in_bits.try_into().unwrap();
        debug!(
            in_bits,
            in_len = self.cmd_buf.len(),
            data = %SpaceHex(&self.cmd_buf),
        );
        shift(&self.handle, 0xa6, in_bits, &self.cmd_buf, buf)?;

        self.cmd_buf.clear();
        self.cmd_read_len = 0;
        self.num_bits = 0;
        self.last_tdi = None;
        self.last_tdo = None;

        Ok(())
    }
}
