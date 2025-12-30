use std::{
    io::{self, Read, Write},
    time::Duration,
};

use rusb::{Direction, Recipient, RequestType, request_type};

type Result<T> = rusb::Result<T>;

pub struct Device {
    handle: rusb::DeviceHandle<rusb::GlobalContext>,
    interface: Interface,
    packet_size: usize,
}

const CHUNK_SIZE: usize = 4096;

impl Write for Device {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let buf = &buf[..buf.len().min(CHUNK_SIZE)];
        self.handle
            .write_bulk(self.interface.endpoints().in_, buf, TIMEOUT)
            .map_err(io::Error::other)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Read for Device {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        // the FTDI device returns packets. The first 2 bytes of every packet are
        // status, and should be dropped.
        let num_packets = buf.len().div_ceil(self.packet_size);

        let mut read_buffer = [0; CHUNK_SIZE];
        let max_read_len = read_buffer.len().min(buf.len() + num_packets * 2);
        let read_buffer = &mut read_buffer[..max_read_len];

        let bytes_read = self
            .handle
            .read_bulk(self.interface.endpoints().out, read_buffer, TIMEOUT)
            .map_err(io::Error::other)?;

        let mut actual_read = 0;
        for packet in read_buffer[..bytes_read].chunks(self.packet_size) {
            let data = &packet[2..];
            let (first, rest) = buf.split_at_mut(data.len());
            first.copy_from_slice(data);
            actual_read += data.len();
            buf = rest;
        }
        Ok(actual_read)
    }
}

#[repr(u8)]
#[derive(Clone, Copy)]
#[allow(unused)]
enum Interface {
    A = 1,
    B = 2,
    C = 3,
    D = 4,
}

struct Endpoints {
    in_: u8,
    out: u8,
}

const FTDI_DEVICE_OUT_REQTYPE: u8 =
    request_type(Direction::Out, RequestType::Vendor, Recipient::Device);

mod requests {
    pub const RESET: u8 = 0;
    pub const SET_FLOW_CTRL: u8 = 2;
    pub const SET_EVENT_CHAR: u8 = 0x06;
    pub const SET_ERROR_CHAR: u8 = 0x07;
    pub const SET_LATENCY_TIMER: u8 = 0x09;
    pub const SET_BITMODE: u8 = 0x0B;
}

const TIMEOUT: Duration = Duration::from_millis(5000);

fn determine_max_packet_size(
    device: rusb::Device<rusb::GlobalContext>,
    interface: Interface,
) -> usize {
    let iface_n = interface.interface().into();
    if let Ok(desc) = device.device_descriptor()
        && desc.num_configurations() > 0
        && let Ok(config) = device.config_descriptor(0)
        && let Some(iface) = config.interfaces().nth(iface_n)
        && let Some(desc) = iface.descriptors().next()
        && desc.num_endpoints() > 0
        && let Some(ep) = desc.endpoint_descriptors().next()
    {
        ep.max_packet_size().into()
    } else {
        // 2232H packet size
        512
    }
}

impl Device {
    pub fn new(handle: rusb::DeviceHandle<rusb::GlobalContext>) -> eyre::Result<Self> {
        let desc = handle.device().device_descriptor()?;
        assert_eq!(
            desc.device_version(),
            rusb::Version(7, 0, 0),
            "only ft2232h supported"
        );

        let interface = Interface::A;
        let packet_size = determine_max_packet_size(handle.device(), interface);

        let _ = handle.detach_kernel_driver(interface.interface());
        handle.claim_interface(interface.interface())?;

        let slf = Self {
            handle,
            packet_size,
            interface,
        };
        slf.init()?;

        Ok(slf)
    }

    fn init(&self) -> Result<()> {
        const RESET_SIO: u16 = 0x00;
        self.write_control(requests::RESET, RESET_SIO)?;
        self.flush_rx()?;
        self.flush_tx()?;
        self.write_control(requests::SET_LATENCY_TIMER, 16)?;

        // high byte is enable, low byte is char (if enabled)
        self.write_control(requests::SET_EVENT_CHAR, 0x00_00)?;
        self.write_control(requests::SET_ERROR_CHAR, 0x00_00)?;

        // flow control is weird for some reason...
        const RTS_CTS: u16 = 0x100;
        let flow_index = RTS_CTS | self.interface.index();
        self.handle.write_control(
            FTDI_DEVICE_OUT_REQTYPE,
            requests::SET_FLOW_CTRL,
            0,
            flow_index,
            &[],
            TIMEOUT,
        )?;

        // high byte is mode, low byte is mask
        self.write_control(requests::SET_BITMODE, 0x02_00)?;

        Ok(())
    }

    /// Flush the read buffer on the chip
    pub fn flush_rx(&self) -> Result<()> {
        const TCI_FLUSH: u16 = 2;
        self.write_control(requests::RESET, TCI_FLUSH)
    }

    /// Flush the write buffer on the chip
    pub fn flush_tx(&self) -> Result<()> {
        const TCO_FLUSH: u16 = 1;
        self.write_control(requests::RESET, TCO_FLUSH)
    }

    fn write_control(&self, request: u8, value: u16) -> Result<()> {
        self.handle.write_control(
            FTDI_DEVICE_OUT_REQTYPE,
            request,
            value,
            self.interface.index(),
            &[],
            TIMEOUT,
        )?;
        Ok(())
    }
}

impl Interface {
    const fn interface(self) -> u8 {
        self as u8 - 1
    }

    const fn index(self) -> u16 {
        self as u8 as u16
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
