use eyre::Result;
pub use ftdi_mpsse::MpsseCmdExecutor;
use tracing::{debug, instrument};

use crate::{
    Backend, Buffer, Hex, SpaceHex, jtag,
    units::{Bits, Bytes},
};

pub mod devices;

pub struct Device {
    pub dev: ::ftdi::Device,
    last: Option<bool>,
    pub cmd_buf: Vec<u8>,
    pub cmd_read_len: usize,
}

// think this is safe? we're sending `&Device`, the only reason it's not
// auto-derived is because `::ftdi::Device` is a `*mut ffi::ftdi_context`. That
// struct doesn't appear to containy any thread-specific constructs.
unsafe impl Send for Device {}

impl Device {
    pub fn new(mut dev: ftdi::Device, info: &devices::Info) -> Result<Self> {
        dev.init(&Default::default())?;
        let init_cmd = [
            MpsseCommand::SetDataBitsLowbyte as u8,
            info.dbus_data,
            info.dbus_en,
            MpsseCommand::SetClockFrequency as u8,
            0x03,
            0x00,
            MpsseCommand::SetDataBitsHighbyte as u8,
            info.cbus_data,
            info.cbus_en,
        ];
        dev.send(&init_cmd)?;

        // TODO: could set the clock to be fast here

        Ok(Self {
            dev,
            last: None,
            cmd_buf: Vec::new(),
            cmd_read_len: 0,
        })
    }
}

#[rustfmt::skip]
mod flags {
    pub const WRITE_NEG : u8 = 0x01;   // Write TDI/DO on negative TCK/SK edge
    pub const BITMODE   : u8 = 0x02;   // Write bits, not bytes
    pub const READ_NEG  : u8 = 0x04;   // Sample TDO/DI on negative TCK/SK edge
    pub const LSB       : u8 = 0x08;   // LSB first
    pub const DO_WRITE  : u8 = 0x10;   // Write TDI/DO
    pub const DO_READ   : u8 = 0x20;   // Read TDO/DI
    pub const WRITE_TMS : u8 = 0x40;   // Write TMS/CS
}
use flags::*;

#[repr(u8)]
#[non_exhaustive]
#[allow(dead_code)]
enum MpsseCommand {
    SetDataBitsLowbyte = 0x80,
    GetDataBitsLowbyte = 0x81,
    SetDataBitsHighbyte = 0x82,
    GetDataBitsHighbyte = 0x83,
    EnableLoopback = 0x84,
    DisableLoopback = 0x85,
    SetClockFrequency = 0x86,
    SendImmediate = 0x87,
    WaitOnIOHigh = 0x88,
    WaitOnIOLow = 0x89,
    DisableClockDivide = 0x8A,
    EnableClockDivide = 0x8B,
    Enable3PhaseClocking = 0x8C,
    Disable3PhaseClocking = 0x8D,
    EnableAdaptiveClocking = 0x96,
    DisableAdaptiveClocking = 0x97,
}

const MAX_READ_WRITE_LEN: usize = u16::MAX as usize + 1;
impl Device {
    fn maybe_flush(&mut self, buf: &mut dyn Buffer) -> Result<()> {
        const MAX_CMD_LEN: usize = MAX_READ_WRITE_LEN;
        if self.cmd_buf.len() >= MAX_CMD_LEN || self.cmd_read_len >= MAX_READ_WRITE_LEN {
            self.flush(buf)?;
        }
        Ok(())
    }
}

// so, weirdness with sending bits when `last` ---
//
// The last bit of information on TDI corresponds with the state transition on
// TMS. When sending a WRITE_TMS, the high bit corresponds to TDI. Therefore,
// last bit 1 -> WRITE_TMS 0b1xxxxxxx, 0 -> WRITE_TMS 0b0xxxxxxx.
impl Backend for Device {
    #[instrument(skip_all)]
    fn tms(&mut self, buf: &mut dyn Buffer, path: jtag::Path) -> Result<()> {
        let tdi = self.last.take().unwrap_or(false);
        debug!(%path, tdi);

        let tdi = if tdi { 0x80 } else { 0x00 };
        let flags = WRITE_TMS | LSB | BITMODE | WRITE_NEG;

        self.cmd_buf.push(flags);
        self.cmd_buf.push(path.len - 1);
        self.cmd_buf.push(tdi | path.as_clocked());

        self.maybe_flush(buf)
    }

    #[instrument(skip_all)]
    fn tdi_bytes(&mut self, buf: &mut dyn Buffer, tdi: &[u8], last: bool) -> Result<()> {
        assert!(self.last.is_none());
        debug!(len = ?Bytes(tdi.len()), last, data = %SpaceHex(tdi));

        let bytes = DO_WRITE | LSB | WRITE_NEG;
        let bits = bytes | BITMODE;

        let (tdi, last) = if last {
            match tdi.split_last() {
                Some((l, data)) => (data, Some(*l)),
                None => (tdi, None),
            }
        } else {
            (tdi, None)
        };

        for chunk in tdi.chunks(MAX_READ_WRITE_LEN) {
            let len = assert_data_len(chunk.len());
            self.cmd_buf.push(bytes);
            self.cmd_buf.push(len as u8);
            self.cmd_buf.push((len >> 8) as u8);
            self.cmd_buf.extend_from_slice(chunk);
            self.maybe_flush(buf)?;
        }

        if let Some(last) = last {
            self.cmd_buf.push(bits);
            // 6 -> 7 bits, tx last bit separately when doing tms
            self.cmd_buf.push(6);
            self.cmd_buf.push(last);
            self.last = Some((last & 0x80) != 0);
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
        assert!(self.last.is_none());
        debug!(?len, last, data = %Hex(tdi));
        let Bits(len) = len;

        assert!(0 < len && len <= 8);
        self.cmd_buf.push(DO_WRITE | LSB | BITMODE | WRITE_NEG);
        self.cmd_buf.push(len as u8 - if last { 2 } else { 1 });
        self.cmd_buf.push(tdi);

        if last {
            self.last = Some(tdi & 1 << (len - 1) != 0);
        }

        self.maybe_flush(buf)
    }

    #[instrument(skip_all)]
    fn tdo_bytes(&mut self, buf: &mut dyn Buffer, len: Bytes<usize>, last: bool) -> Result<()> {
        assert!(self.last.is_none());
        debug!(?len, last);
        let Bytes(mut len) = len;

        while len != 0 {
            let to_add = len.min(MAX_READ_WRITE_LEN);
            self.cmd_read_len += to_add;
            let read_len = assert_data_len(to_add);

            self.cmd_buf.push(DO_READ | LSB | READ_NEG | WRITE_NEG);
            self.cmd_buf.push(read_len as u8);
            self.cmd_buf.push((read_len >> 8) as u8);

            len = len.saturating_sub(MAX_READ_WRITE_LEN);
            self.maybe_flush(buf)?;
        }

        Ok(())
    }

    #[instrument(skip_all)]
    fn tdi_tdo_bytes(&mut self, buf: &mut dyn Buffer, tdi: &[u8], last: bool) -> Result<()> {
        assert!(self.last.is_none());
        debug!(len = ?Bytes(tdi.len()), last, data = %SpaceHex(tdi));

        let bytes = DO_READ | DO_WRITE | LSB | READ_NEG | WRITE_NEG;
        let bits = bytes | BITMODE;

        let (tdi, last) = if last {
            match tdi.split_last() {
                Some((l, data)) => (data, Some(*l)),
                None => (tdi, None),
            }
        } else {
            (tdi, None)
        };

        for chunk in tdi.chunks(MAX_READ_WRITE_LEN) {
            self.cmd_read_len += chunk.len();
            let len = assert_data_len(chunk.len());
            self.cmd_buf.push(bytes);
            self.cmd_buf.push(len as u8);
            self.cmd_buf.push((len >> 8) as u8);
            self.cmd_buf.extend_from_slice(chunk);
            self.maybe_flush(buf)?;
        }

        if let Some(last) = last {
            self.cmd_buf.push(bits);
            // 6 -> 7 bits, tx last bit separately when doing tms
            self.cmd_buf.push(6);
            self.cmd_buf.push(last);
            self.cmd_read_len += 1;
            self.last = Some((last & 0x80) != 0);
        }

        Ok(())
    }

    #[instrument(skip_all)]
    fn flush(&mut self, buf: &mut dyn Buffer) -> Result<()> {
        self.cmd_buf.push(MpsseCommand::SendImmediate as u8);
        debug!(
            write_len = self.cmd_buf.len(),
            read_len = self.cmd_read_len,
            data = %SpaceHex(&self.cmd_buf),
        );

        let buf = buf.extend(self.cmd_read_len);
        self.dev.xfer(&self.cmd_buf, buf)?;

        self.cmd_buf.clear();
        self.cmd_read_len = 0;
        Ok(())
    }
}

fn assert_data_len(len: usize) -> u16 {
    assert!(len != 0);
    assert!(
        len <= u16::MAX as usize + 1,
        "data length cannot exceed u16::MAX + 1"
    );
    (len - 1) as u16
}
