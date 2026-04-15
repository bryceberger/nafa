use std::time::Duration;

use eyre::Result;
use nusb::transfer::{ControlIn, ControlType, Recipient};

use crate::{
    Backend, Buffer, Data, jtag,
    units::{Bits, Bytes},
};

pub mod firmware;
mod io;

pub struct Device {
    dev: io::Device,
    cmd_buf: Vec<u8>,
    read_buf: Vec<Read>,
}

mod bitbang {
    pub const TCK: u8 = 1 << 0;
    pub const TMS: u8 = 1 << 1;
    pub const TDI: u8 = 1 << 4;
    pub const TDO: u8 = 1 << 6;

    pub const DEFAULT: u8 = 1 << 2 | 1 << 3 | 1 << 5;
}

#[derive(Clone, Copy)]
enum Read {
    Bytes(u8),
    Bits,
}

const MAX_READ_WRITE_LEN: usize = 0b111111;
fn bytes_header(read: bool, Bytes(len): Bytes<u8>) -> u8 {
    assert!(len <= MAX_READ_WRITE_LEN as u8);
    let byte_mode = 1 << 7;
    let read = (read as u8) << 6;
    byte_mode | read | len
}

impl Device {
    pub async fn new(handle: nusb::Device) -> Result<Self> {
        const READ_REV: u8 = 0x94;
        let iface = handle.claim_interface(0).await?;
        let data = ControlIn {
            control_type: ControlType::Vendor,
            recipient: Recipient::Device,
            request: READ_REV,
            value: 0,
            index: 0,
            length: 5,
        };
        let rev = iface.control_in(data, Duration::from_secs(1)).await?;
        tracing::info!(rev = %crate::ShortHex(&rev));
        tracing::info!(rev = ?str::from_utf8(&rev));

        Ok(Self {
            dev: io::Device { iface },
            cmd_buf: Vec::new(),
            read_buf: Vec::new(),
        })
    }

    async fn bits_internal(
        &mut self,
        buf: &mut dyn Buffer,
        before: Option<jtag::Path>,
        mut data: u32,
        len: Bits<u8>,
        after: Option<jtag::Path>,
        read: bool,
    ) -> Result<()> {
        if let Some(path) = before {
            for tms in path {
                self.add_bit(tms, true, false);
            }
        }

        let len = match after {
            Some(_) => len.0 - 1,
            None => len.0,
        };

        let tms = false;
        for _ in 0..len {
            self.add_bit(tms, data & 1 == 1, read);
            data >>= 1;
        }

        if let Some(path) = after {
            let mut it = path.into_iter();
            if let Some(tms) = it.next() {
                self.add_bit(tms, data & 1 == 1, read);
            }
            for tms in it {
                self.add_bit(tms, true, false);
            }
        }

        self.maybe_flush(buf).await?;
        Ok(())
    }

    fn add_bit(&mut self, tms: bool, tdi: bool, tdo: bool) {
        let tms = if tms { bitbang::TMS } else { 0 };
        let tdi = if tdi { bitbang::TDI } else { 0 };
        let tdo = if tdo { bitbang::TDO } else { 0 };
        let cmd = tms | tdi | bitbang::DEFAULT;
        self.cmd_buf.push(cmd);
        self.cmd_buf.push(cmd | tdo | bitbang::TCK);
    }

    fn ensure_tck_low(&mut self) {
        self.cmd_buf.push(bitbang::DEFAULT);
        self.cmd_buf.push(bitbang::DEFAULT);
    }

    async fn maybe_flush(&mut self, buf: &mut dyn Buffer) -> Result<()> {
        const MAX_BUF_LEN: usize = 8192;
        if self.cmd_buf.len() >= MAX_BUF_LEN {
            self.flush(buf).await?;
        }
        Ok(())
    }
}

#[async_trait::async_trait]
impl Backend for Device {
    async fn tms(&mut self, buf: &mut dyn Buffer, path: jtag::Path) -> Result<()> {
        for tms in path {
            self.add_bit(tms, true, false);
        }
        self.maybe_flush(buf).await?;
        Ok(())
    }

    async fn bits(
        &mut self,
        buf: &mut dyn Buffer,
        before: Option<jtag::Path>,
        data: u32,
        len: Bits<u8>,
        after: Option<jtag::Path>,
    ) -> Result<()> {
        self.bits_internal(buf, before, data, len, after, false)
            .await
    }

    async fn bytes(
        &mut self,
        buf: &mut dyn Buffer,
        before: Option<jtag::Path>,
        data: Data<'_>,
        after: Option<jtag::Path>,
    ) -> Result<()> {
        if let Some(path) = before {
            let tdi = true;
            let tdo = false;
            for tms in path {
                self.add_bit(tms, tdi, tdo);
            }
        }

        self.ensure_tck_low();
        match data {
            Data::Tx(tdi) | Data::TxRx(tdi) => {
                let read = matches!(data, Data::TxRx(_));

                let (tdi, last) = match (after, tdi.split_last()) {
                    (Some(_), Some((l, data))) => (data, Some(*l)),
                    (None, _) => (tdi, None),
                    (Some(_), None) => panic!("cannot have path after with zero length data"),
                };

                for chunk in tdi.chunks(MAX_READ_WRITE_LEN) {
                    let to_add = chunk.len() as u8;
                    self.cmd_buf.push(bytes_header(read, Bytes(to_add)));
                    self.cmd_buf.extend_from_slice(chunk);
                    buf.notify_write(chunk.len());
                    if read {
                        self.read_buf.push(Read::Bytes(to_add))
                    }
                    self.maybe_flush(buf).await?;
                }

                if let Some(last) = last {
                    buf.notify_write(1);
                    if read {
                        self.read_buf.push(Read::Bits);
                    }
                    self.bits_internal(buf, None, last.into(), Bits(8), after, read)
                        .await?;
                }
            }
            Data::Rx(Bytes(mut len)) | Data::ConstantTx(_, Bytes(mut len)) => {
                let read = matches!(data, Data::Rx(_));
                let tdi = match data {
                    Data::ConstantTx(tdi, _) => tdi,
                    _ => true,
                };
                static ZEROES: &[u8; MAX_READ_WRITE_LEN] = &[0x00; MAX_READ_WRITE_LEN];
                static ONES: &[u8; MAX_READ_WRITE_LEN] = &[0xff; MAX_READ_WRITE_LEN];
                let tdi_data = if tdi { ONES } else { ZEROES };

                while len != 0 {
                    let to_add = len.min(MAX_READ_WRITE_LEN) as u8;

                    self.cmd_buf.push(bytes_header(read, Bytes(to_add)));
                    self.cmd_buf.extend_from_slice(&tdi_data[..to_add.into()]);

                    if read {
                        self.read_buf.push(Read::Bytes(to_add));
                    }

                    len = len.saturating_sub(MAX_READ_WRITE_LEN);
                    self.maybe_flush(buf).await?;
                }

                if let Some(after) = after {
                    for tms in after {
                        self.add_bit(tms, tdi, read);
                    }
                }
            }
        }

        self.flush(buf).await?;
        Ok(())
    }

    async fn flush(&mut self, buf: &mut dyn Buffer) -> Result<()> {
        let it = self.read_buf.iter().copied().map(|r| match r {
            Read::Bytes(len) => len.into(),
            Read::Bits => 1,
        });
        let total_read_len: usize = it.sum();

        let buf = buf.extend(total_read_len, 0);
        self.dev.write(&self.cmd_buf).await?;
        // TODO: can this be better now that `Buffer` has the scratch param?
        self.dev
            .do_reads(buf, self.read_buf.iter().copied())
            .await?;

        self.cmd_buf.clear();
        self.read_buf.clear();
        Ok(())
    }
}
