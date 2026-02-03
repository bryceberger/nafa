use std::time::Duration;

use eyre::Result;
use nusb::transfer::{self, ControlOut, ControlType, Recipient};

use crate::ftdi::devices::Interface;

pub struct Device {
    iface: nusb::Interface,
    endpoints: Endpoints,
    packet_size: usize,
}

const CHUNK_SIZE: usize = 1024;

struct Endpoints {
    in_: u8,
    out: u8,
}

mod requests {
    pub const RESET: u8 = 0;
    pub const SET_FLOW_CTRL: u8 = 2;
    pub const SET_EVENT_CHAR: u8 = 0x06;
    pub const SET_ERROR_CHAR: u8 = 0x07;
    pub const SET_LATENCY_TIMER: u8 = 0x09;
    pub const SET_BITMODE: u8 = 0x0B;
}

const TIMEOUT: Duration = Duration::from_millis(5000);

#[tracing::instrument(skip_all)]
fn determine_max_packet_size(iface: &nusb::Interface) -> usize {
    if let Some(desc) = iface.descriptor()
        && let Some(ep) = desc.endpoints().next()
    {
        ep.max_packet_size()
    } else {
        // 2232H packet size
        512
    }
}

impl Device {
    #[tracing::instrument(skip_all)]
    pub async fn new(handle: nusb::Device, interface: Interface) -> Result<Self> {
        let desc = handle.device_descriptor();
        assert_eq!(desc.device_version(), 0x0700, "only ft2232h supported");

        let _ = handle.detach_kernel_driver(interface.interface());
        let iface = handle.claim_interface(interface.interface()).await?;
        let packet_size = determine_max_packet_size(&iface);

        let slf = Self {
            iface,
            packet_size,
            endpoints: interface.endpoints(),
        };
        slf.init().await?;

        Ok(slf)
    }

    #[tracing::instrument(skip_all)]
    async fn init(&self) -> Result<()> {
        const RESET_SIO: u16 = 0x00;
        self.write_control(requests::RESET, RESET_SIO).await?;
        self.flush_rx().await?;
        self.flush_tx().await?;
        self.write_control(requests::SET_LATENCY_TIMER, 16).await?;

        // high byte is enable, low byte is char (if enabled)
        self.write_control(requests::SET_EVENT_CHAR, 0x00_00)
            .await?;
        self.write_control(requests::SET_ERROR_CHAR, 0x00_00)
            .await?;

        // flow control is weird for some reason...
        const RTS_CTS: u16 = 0x100;
        let flow_index = RTS_CTS | (u16::from(self.iface.interface_number()) + 1);
        let data = ControlOut {
            control_type: ControlType::Vendor,
            recipient: Recipient::Device,
            request: requests::SET_FLOW_CTRL,
            value: 0,
            index: flow_index,
            data: &[],
        };
        self.iface.control_out(data, TIMEOUT).await?;

        // high byte is mode, low byte is mask
        self.write_control(requests::SET_BITMODE, 0x02_00).await?;

        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn send(&mut self, data: &[u8]) -> Result<()> {
        use futures_lite::AsyncWriteExt;
        self.flush_rx().await?;
        let mut writer = self
            .iface
            .endpoint::<transfer::Bulk, transfer::Out>(self.endpoints.in_)?
            .writer(CHUNK_SIZE)
            .with_write_timeout(TIMEOUT);
        tracing::info!(len = %data.len(), buf = %crate::SpaceHex(data), "writing");
        writer.write_all(data).await?;
        writer.flush().await?;
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn recv(&mut self, mut buf: &mut [u8]) -> Result<usize> {
        use futures_lite::AsyncReadExt;

        let original_len = buf.len();

        let mut reader = self
            .iface
            .endpoint::<transfer::Bulk, transfer::In>(self.endpoints.out)?
            .reader(CHUNK_SIZE)
            .with_read_timeout(TIMEOUT);

        let mut read_buffer = [0; CHUNK_SIZE];

        // the FTDI device returns packets. The first 2 bytes of every packet are
        // status, and should be dropped.
        let num_packets = buf.len().div_ceil(self.packet_size);
        let max_read_len = read_buffer.len().min(buf.len() + num_packets * 2);
        let read_buffer = &mut read_buffer[..max_read_len];

        let mut actual_bytes_read = 0;
        while !buf.is_empty() {
            tracing::info!(len = %read_buffer.len(), buf = %crate::SpaceHex(buf), "reading");
            let bytes_read = reader.read(read_buffer).await?;
            tracing::info!(bytes_read, read = %crate::SpaceHex(&read_buffer[..bytes_read]));
            if bytes_read <= 2 {
                break;
            }
            for packet in read_buffer[..bytes_read].chunks(self.packet_size) {
                let data = &packet[2..];
                let (first, rest) = buf.split_at_mut(data.len());
                first.copy_from_slice(data);
                actual_bytes_read += data.len();
                buf = rest;
            }
        }

        if actual_bytes_read != original_len {
            return Err(eyre::eyre!(
                "failed to fill buffer: read {actual_bytes_read} bytes, expected {original_len}"
            ));
        }

        Ok(actual_bytes_read)
    }

    /// Flush the read buffer on the chip
    #[tracing::instrument(skip_all)]
    pub async fn flush_rx(&self) -> Result<()> {
        const TCI_FLUSH: u16 = 2;
        self.write_control(requests::RESET, TCI_FLUSH).await
    }

    /// Flush the write buffer on the chip
    #[tracing::instrument(skip_all)]
    pub async fn flush_tx(&self) -> Result<()> {
        const TCO_FLUSH: u16 = 1;
        self.write_control(requests::RESET, TCO_FLUSH).await
    }

    #[tracing::instrument(skip(self))]
    async fn write_control(&self, request: u8, value: u16) -> Result<()> {
        let data = ControlOut {
            control_type: ControlType::Vendor,
            recipient: Recipient::Device,
            request,
            value,
            index: u16::from(self.iface.interface_number()) + 1,
            data: &[],
        };
        self.iface.control_out(data, TIMEOUT).await?;
        Ok(())
    }
}

impl Interface {
    const fn interface(self) -> u8 {
        self as u8
    }

    const fn endpoints(self) -> Endpoints {
        match self {
            Interface::A => Endpoints {
                in_: 0x02,
                out: 0x81,
            },
            Interface::B => Endpoints {
                in_: 0x04,
                out: 0x83,
            },
            Interface::C => Endpoints {
                in_: 0x06,
                out: 0x85,
            },
            Interface::D => Endpoints {
                in_: 0x08,
                out: 0x87,
            },
        }
    }
}
