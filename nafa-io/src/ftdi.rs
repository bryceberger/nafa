use eyre::Result;
pub use ftdi_mpsse::MpsseCmdExecutor;
use tracing::{debug, instrument};

use crate::{
    Backend, Buffer, SpaceHex,
    backend::Data,
    jtag,
    units::{Bits, Bytes},
};

pub mod devices;

pub struct Device {
    dev: ::ftdi::Device,
    cmd_buf: Vec<u8>,
    cmd_read_len: usize,
}

impl Device {
    pub fn new(
        mut dev: ftdi::Device,
        info: &devices::Info,
        clock_frequency: Option<u32>,
    ) -> Result<Self> {
        dev.init(&ftdi_mpsse::MpsseSettings {
            clock_frequency,
            ..Default::default()
        })?;
        let init_cmd = [
            MpsseCommand::SetDataBitsLowbyte as u8,
            info.dbus_data,
            info.dbus_en,
            MpsseCommand::SetDataBitsHighbyte as u8,
            info.cbus_data,
            info.cbus_en,
        ];
        dev.send(&init_cmd)?;

        Ok(Self {
            dev,
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

    fn tms_internal(&mut self, buf: &mut dyn Buffer, path: jtag::Path, tdi: bool) -> Result<()> {
        debug!(%path, tdi);

        let tdi = if tdi { 0x80 } else { 0x00 };
        let flags = WRITE_TMS | LSB | BITMODE | WRITE_NEG;

        self.cmd_buf.push(flags);
        self.cmd_buf.push(path.len - 1);
        self.cmd_buf.push(tdi | path.as_clocked());

        self.maybe_flush(buf)
    }
}

static ONES: &[u8; MAX_READ_WRITE_LEN] = &[0xff; MAX_READ_WRITE_LEN];
static ZEROES: &[u8; MAX_READ_WRITE_LEN] = &[0x00; MAX_READ_WRITE_LEN];

// so, weirdness with sending bits when `last` ---
//
// The last bit of information on TDI corresponds with the state transition on
// TMS. When sending a WRITE_TMS, the high bit corresponds to TDI. Therefore,
// last bit 1 -> WRITE_TMS 0b1xxxxxxx, 0 -> WRITE_TMS 0b0xxxxxxx.
impl Backend for Device {
    #[instrument(skip_all)]
    fn tms(&mut self, buf: &mut dyn Buffer, path: jtag::Path) -> Result<()> {
        self.tms_internal(buf, path, true)
    }

    #[instrument(skip_all)]
    fn bytes(
        &mut self,
        buf: &mut dyn Buffer,
        before: Option<jtag::Path>,
        data: Data<'_>,
        after: Option<jtag::Path>,
    ) -> Result<()> {
        if let Some(path) = before {
            self.tms_internal(buf, path, true)?;
        }

        let mut last_bit = true;

        match data {
            Data::Tx(tdi) | Data::TxRx(tdi) => {
                let read = matches!(data, Data::TxRx(_));
                let read_cmd = if read { DO_READ | READ_NEG } else { 0 };
                let cmd = read_cmd | DO_WRITE | LSB | WRITE_NEG;

                let (tdi, last) = match (after, tdi.split_last()) {
                    (Some(_), Some((l, data))) => (data, Some(*l)),
                    (None, _) | (Some(_), None) => (tdi, None),
                };

                for chunk in tdi.chunks(MAX_READ_WRITE_LEN) {
                    if read {
                        self.cmd_read_len += chunk.len();
                    }
                    let len = assert_data_len(chunk.len());
                    self.cmd_buf.push(cmd);
                    self.cmd_buf.push(len as u8);
                    self.cmd_buf.push((len >> 8) as u8);
                    self.cmd_buf.extend_from_slice(chunk);
                    buf.notify_write(chunk.len());
                    self.maybe_flush(buf)?;
                }

                if let Some(last) = last {
                    self.cmd_buf.push(cmd | BITMODE);
                    // 7 bits, tx last bit as part of tms
                    self.cmd_buf.push(6);
                    self.cmd_buf.push(last);
                    if read {
                        self.cmd_read_len += 1;
                    }
                    buf.notify_write(1);
                    last_bit = last & 0x80 != 0;
                }
            }
            Data::Rx(Bytes(mut len)) => {
                while len != 0 {
                    let to_add = len.min(MAX_READ_WRITE_LEN);
                    self.cmd_read_len += to_add;
                    let read_len = assert_data_len(to_add);

                    self.cmd_buf.push(DO_READ | LSB | READ_NEG);
                    self.cmd_buf.push(read_len as u8);
                    self.cmd_buf.push((read_len >> 8) as u8);

                    len = len.saturating_sub(MAX_READ_WRITE_LEN);
                    self.maybe_flush(buf)?;
                }
            }
            Data::ConstantTx(tdi, Bytes(mut len)) => {
                while len != 0 {
                    let to_add = len.min(MAX_READ_WRITE_LEN);
                    let tdi = if tdi {
                        &ONES[..to_add]
                    } else {
                        &ZEROES[..to_add]
                    };
                    let write_len = assert_data_len(to_add);

                    self.cmd_buf.push(DO_WRITE | LSB | WRITE_NEG);
                    self.cmd_buf.push(write_len as u8);
                    self.cmd_buf.push((write_len >> 8) as u8);
                    self.cmd_buf.extend_from_slice(tdi);

                    len = len.saturating_sub(MAX_READ_WRITE_LEN);
                    self.maybe_flush(buf)?;
                }
            }
        };

        if let Some(path) = after {
            self.tms_internal(buf, path, last_bit)?;
        }

        self.maybe_flush(buf)?;
        Ok(())
    }

    #[instrument(skip_all)]
    fn bits(
        &mut self,
        buf: &mut dyn Buffer,
        before: Option<jtag::Path>,
        mut data: u32,
        len: Bits<u8>,
        after: Option<jtag::Path>,
    ) -> Result<()> {
        if let Some(path) = before {
            self.tms_internal(buf, path, true)?;
        }

        let mut len = match after {
            Some(_) => len.0 - 1,
            None => len.0,
        };

        let cmd = DO_WRITE | LSB | WRITE_NEG | BITMODE;
        while len != 0 {
            let added = if len > 8 { 8 } else { len };
            self.cmd_buf.push(cmd);
            self.cmd_buf.push(added - 1);
            self.cmd_buf.push(data as u8);
            data >>= added;
            len -= added;
        }

        if let Some(path) = after {
            self.tms_internal(buf, path, data & 1 == 1)?;
        }

        self.maybe_flush(buf)?;
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
