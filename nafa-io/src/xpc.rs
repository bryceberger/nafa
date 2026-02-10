use std::time::Duration;

use eyre::Result;
use nusb::transfer::{self, ControlIn, ControlOut, ControlType, Recipient};
use tracing::{info, instrument};

use crate::{Backend, Buffer, Hex, SpaceHex, backend::Data, jtag, units::Bits};

pub mod firmware;

pub struct Device {
    handle: nusb::Device,
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

pub async fn flash(dev: &nusb::Device, firmware: &[(u16, &[u8])]) -> Result<()> {
    // A host loader program must write 0x01 to the CPUCS register
    // to put the CPU into RESET, load all or part of the EZUSB
    // RAM with firmware, then reload the CPUCS register
    // with ‘0’ to take the CPU out of RESET. The CPUCS register
    // (at 0xE600) is the only EZ-USB register that can be written
    // using the Firmware Download command.
    let iface = dev.claim_interface(0).await?;
    let request = XPCU_CTRL_LOAD_FIRM;
    let index = 0;
    let packet = ControlOut {
        control_type: ControlType::Vendor,
        recipient: Recipient::Device,
        request,
        value: EZUSB_CPUCS,
        index,
        data: &[CPU_RESET],
    };
    iface.control_out(packet, S).await?;

    for (addr, data) in firmware {
        let mut addr = *addr;
        for chunk in data.chunks(64) {
            let data = ControlOut {
                value: addr,
                data: chunk,
                ..packet
            };
            iface.control_out(data, S).await?;
            addr += u16::try_from(chunk.len()).unwrap();
        }
    }

    let data = ControlOut {
        data: &[0],
        ..packet
    };
    iface.control_out(data, S).await?;

    Ok(())
}

impl Device {
    pub async fn new(h: nusb::Device) -> Result<Self> {
        request_28(&h, 0x11).await?;
        write_gpio(&h, XPC_PROG).await?;
        info!(
            firmware_version = %Hex(read_firmware_version(&h).await?),
            cpld_version = %Hex(read_cpld_version(&h).await?),
        );

        request_28(&h, 0x11).await?;
        output_enable(&h, true).await?;
        shift(&h, 0xa6, 2, &[0x00; 2], None).await?;
        request_28(&h, 0x12).await?;

        Ok(Self {
            handle: h,
            cmd_buf: Vec::new(),
            cmd_read_len: 0,
            num_bits: 0,
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

    async fn maybe_flush(&mut self, buf: &mut dyn Buffer) -> Result<()> {
        const MAX_BUF_LEN: usize = 8192;
        if self.cmd_buf.len() >= MAX_BUF_LEN {
            self.flush(buf).await?;
        }
        Ok(())
    }
}

async fn request_28(h: &nusb::Device, index: u16) -> Result<()> {
    let data = ControlOut {
        control_type: ControlType::Vendor,
        recipient: Recipient::Device,
        request: 0xb0,
        value: 0x0028,
        index,
        data: &[],
    };
    h.control_out(data, S).await?;
    Ok(())
}

async fn write_gpio(h: &nusb::Device, bits: u16) -> Result<()> {
    let data = ControlOut {
        control_type: ControlType::Vendor,
        recipient: Recipient::Device,
        request: 0xb0,
        value: 0x0030,
        index: bits,
        data: &[],
    };
    h.control_out(data, S).await?;
    Ok(())
}

async fn read_firmware_version(h: &nusb::Device) -> Result<u16> {
    let data = ControlIn {
        control_type: ControlType::Vendor,
        recipient: Recipient::Device,
        request: 0xb0,
        value: 0x0050,
        index: 0x0000,
        length: std::mem::size_of::<u16>() as _,
    };
    let buf = h.control_in(data, S).await?;
    Ok(u16::from_le_bytes(buf.try_into().unwrap()))
}

async fn read_cpld_version(h: &nusb::Device) -> Result<u16> {
    let data = ControlIn {
        control_type: ControlType::Vendor,
        recipient: Recipient::Device,
        request: 0xb0,
        value: 0x0050,
        index: 0x0001,
        length: std::mem::size_of::<u16>() as _,
    };
    let buf = h.control_in(data, S).await?;
    Ok(u16::from_le_bytes(buf.try_into().unwrap()))
}

async fn output_enable(h: &nusb::Device, enable: bool) -> Result<()> {
    let data = ControlOut {
        control_type: ControlType::Vendor,
        recipient: Recipient::Device,
        request: 0xb0,
        value: if enable { 0x18 } else { 0x10 },
        index: 0,
        data: &[],
    };
    h.control_out(data, S).await?;
    Ok(())
}

async fn shift(
    h: &nusb::Device,
    reqno: u16,
    bits: u16,
    in_buf: &[u8],
    out_buf: Option<&mut [u8]>,
) -> Result<()> {
    use futures_lite::{AsyncReadExt, AsyncWriteExt};

    let data = ControlOut {
        control_type: ControlType::Vendor,
        recipient: Recipient::Device,
        request: 0xb0,
        value: reqno,
        index: bits,
        data: &[],
    };
    h.control_out(data, S).await?;

    let iface = h.claim_interface(0).await?;

    let mut writer = iface
        .endpoint::<transfer::Bulk, transfer::Out>(0x02)?
        .writer(128)
        .with_num_transfers(8)
        .with_write_timeout(S);
    writer.write_all(in_buf).await?;
    writer.flush().await?;

    if let Some(out) = out_buf {
        iface
            .endpoint::<transfer::Bulk, transfer::In>(0x86)?
            .reader(128)
            .with_num_transfers(8)
            .with_read_timeout(S)
            .read_exact(out)
            .await?;
    }

    Ok(())
}

#[async_trait::async_trait]
impl Backend for Device {
    #[instrument(skip_all)]
    async fn tms(&mut self, buf: &mut dyn Buffer, path: jtag::Path) -> Result<()> {
        for tms in path {
            self.add_bit(tms, true, false);
        }

        self.maybe_flush(buf).await?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn bytes(
        &mut self,
        buf: &mut dyn Buffer,
        before: Option<jtag::Path>,
        data: Data<'_>,
        after: Option<jtag::Path>,
    ) -> Result<()> {
        if let Some(path) = before {
            for tms in path {
                self.add_bit(tms, true, false);
            }
        }

        // without this, reads (sometimes???) drop the last few bits
        while self.num_bits != 0 {
            self.add_bit_internal(false, false, false, false);
        }

        let tms = false;
        let mut last_tdi = true;
        let mut last_tdo = false;
        match data {
            Data::Tx(tdi) | Data::TxRx(tdi) => {
                let tdo = matches!(data, Data::TxRx(_));
                for (idx, byte) in tdi.iter().copied().enumerate() {
                    self.add_bit(tms, byte & 1 != 0, tdo);
                    self.add_bit(tms, byte >> 1 & 1 != 0, tdo);
                    self.add_bit(tms, byte >> 2 & 1 != 0, tdo);
                    self.add_bit(tms, byte >> 3 & 1 != 0, tdo);
                    self.add_bit(tms, byte >> 4 & 1 != 0, tdo);
                    self.add_bit(tms, byte >> 5 & 1 != 0, tdo);
                    self.add_bit(tms, byte >> 6 & 1 != 0, tdo);
                    if after.is_some() && idx == tdi.len() - 1 {
                        last_tdi = byte >> 7 & 1 != 0;
                        last_tdo = tdo;
                    } else {
                        self.add_bit(tms, byte >> 7 & 1 != 0, tdo);
                    }

                    if tdo {
                        self.cmd_read_len += 1;
                    }
                    buf.notify_write(1);
                    self.maybe_flush(buf).await?;
                }
            }
            Data::Rx(len) | Data::ConstantTx(_, len) => {
                let tdi = match data {
                    Data::ConstantTx(tdi, _) => tdi,
                    _ => true,
                };
                let tdo = matches!(data, Data::Rx(_));
                for idx in 0..len.0 {
                    self.add_bit(tms, tdi, tdo);
                    self.add_bit(tms, tdi, tdo);
                    self.add_bit(tms, tdi, tdo);
                    self.add_bit(tms, tdi, tdo);
                    self.add_bit(tms, tdi, tdo);
                    self.add_bit(tms, tdi, tdo);
                    self.add_bit(tms, tdi, tdo);
                    if after.is_some() && idx == len.0 - 1 {
                        last_tdo = tdo;
                    } else {
                        self.add_bit(tms, tdi, tdo);
                    }

                    if tdo {
                        self.cmd_read_len += 1;
                    }
                    self.maybe_flush(buf).await?;
                }
            }
        }

        if let Some(path) = after {
            let mut it = path.into_iter();
            if let Some(tms) = it.next() {
                self.add_bit(tms, last_tdi, last_tdo);
            }
            for tms in it {
                self.add_bit(tms, true, false);
            }
        }

        self.maybe_flush(buf).await?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn bits(
        &mut self,
        buf: &mut dyn Buffer,
        before: Option<jtag::Path>,
        mut data: u32,
        len: Bits<u8>,
        after: Option<jtag::Path>,
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
            self.add_bit(tms, data & 1 == 1, false);
            data >>= 1;
        }

        if let Some(path) = after {
            let mut it = path.into_iter();
            if let Some(tms) = it.next() {
                self.add_bit(tms, data & 1 == 1, false);
            }
            for tms in it {
                self.add_bit(tms, true, false);
            }
        }

        self.maybe_flush(buf).await?;
        Ok(())
    }

    #[instrument(skip_all)]
    async fn flush(&mut self, buf: &mut dyn Buffer) -> Result<()> {
        if self.num_bits == 0 {
            self.add_bit_internal(false, false, false, false);
        }

        let buf = match self.cmd_read_len {
            0 => None,
            _ => Some(buf.extend(self.cmd_read_len)),
        };
        let in_bits = (self.cmd_buf.len() - 2) / 2 * 4 + usize::from(self.num_bits);
        let in_bits = in_bits.try_into().unwrap();
        info!(
            in_bits,
            in_len = self.cmd_buf.len(),
            expect_read = self.cmd_read_len,
            data = %SpaceHex(&self.cmd_buf),
        );
        shift(&self.handle, 0xa6, in_bits, &self.cmd_buf, buf).await?;

        self.cmd_buf.clear();
        self.cmd_read_len = 0;
        self.num_bits = 0;

        Ok(())
    }
}
