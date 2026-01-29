use std::pin::Pin;

use eyre::Result;

use crate::{Backend, ftdi, xpc};

type BoxedBackend = Box<dyn Backend>;
type InitResult = Pin<Box<dyn Future<Output = Result<BoxedBackend>>>>;
type InitFn = fn(nusb::Device) -> InitResult;

pub async fn init(device: nusb::DeviceInfo) -> Result<BoxedBackend, Vec<eyre::Report>> {
    let mut errs = Vec::new();

    for cable in KNOWN {
        if cable.vid == device.vendor_id() && cable.pid == device.product_id() {
            let device = device.open().await.map_err(|x| vec![x.into()])?;
            match (cable.init)(device).await {
                Ok(backend) => return Ok(backend),
                Err(e) => errs.push(e.wrap_err(eyre::eyre!("while trying cable {}", cable.name))),
            }
        }
    }

    Err(errs)
}

pub struct Cable {
    pub name: &'static str,
    pub vid: u16,
    pub pid: u16,
    pub init: InitFn,
}

fn init_ftdi(
    device: nusb::Device,
    info: &'static ftdi::devices::Info,
    clock_frequency: u32,
) -> InitResult {
    Box::pin(async move {
        Ok(Box::new(ftdi::Device::new(device, info, clock_frequency).await?) as BoxedBackend)
    })
}

const fn c(vid: u16, pid: u16, name: &'static str, init: InitFn) -> Cable {
    Cable {
        name,
        vid,
        pid,
        init,
    }
}

pub const KNOWN: &[Cable] = &[
    c(0x0403, 0xcff8, "amontec", |device| {
        init_ftdi(device, &ftdi::devices::AMONTEC, 1500000)
    }),
    c(0x15ba, 0x002b, "arm-usb-ocd-h", |device| {
        init_ftdi(device, &ftdi::devices::ARM_USB_OCD_H, 1500000)
    }),
    c(0x0403, 0x6010, "bbv2", |device| {
        init_ftdi(device, &ftdi::devices::BBV2, 1500000)
    }),
    c(0x0403, 0x6010, "bbv2_2", |device| {
        init_ftdi(device, &ftdi::devices::BBV2_2, 1500000)
    }),
    c(0x0403, 0x8350, "cm1", |device| {
        init_ftdi(device, &ftdi::devices::CM1, 1500000)
    }),
    c(0x0403, 0x6010, "dlp2232h", |device| {
        init_ftdi(device, &ftdi::devices::DLP2232H, 1500000)
    }),
    c(0x0403, 0x6010, "ft2232test", |device| {
        init_ftdi(device, &ftdi::devices::FT2232TEST, 8000000)
    }),
    c(0x0403, 0x6011, "ft4232h", |device| {
        init_ftdi(device, &ftdi::devices::FT4232H, 1500000)
    }),
    c(0x0403, 0x6010, "ftdijtag", |device| {
        init_ftdi(device, &ftdi::devices::FTDIJTAG, 1500000)
    }),
    c(0x0403, 0x6010, "ikda", |device| {
        init_ftdi(device, &ftdi::devices::IKDA, 1500000)
    }),
    c(0x0403, 0x6014, "jtaghs2", |device| {
        init_ftdi(device, &ftdi::devices::JTAGHS2, 6000000)
    }),
    c(0x0403, 0x6010, "l_motctl", |device| {
        init_ftdi(device, &ftdi::devices::L_MOTCTL, 8000000)
    }),
    c(0x0403, 0x6010, "llbbc", |device| {
        init_ftdi(device, &ftdi::devices::LLBBC, 8000000)
    }),
    c(0x0403, 0x6010, "llbus", |device| {
        init_ftdi(device, &ftdi::devices::LLBUS, 1500000)
    }),
    c(0x0403, 0x6010, "llif", |device| {
        init_ftdi(device, &ftdi::devices::LLIF, 8000000)
    }),
    c(0x2A19, 0x1009, "mimas_a7", |device| {
        init_ftdi(device, &ftdi::devices::MIMAS_A7, 15000000)
    }),
    c(0x0403, 0x6010, "nexys4", |device| {
        init_ftdi(device, &ftdi::devices::NEXYS4, 30000000)
    }),
    c(0x15b1, 0x0003, "olimex", |device| {
        init_ftdi(device, &ftdi::devices::OLIMEX, 1500000)
    }),
    c(0x9e88, 0x9e8f, "plugjtag", |device| {
        init_ftdi(device, &ftdi::devices::PLUGJTAG, 1500000)
    }),
    c(0x0403, 0x8a98, "tumpa", |device| {
        init_ftdi(device, &ftdi::devices::TUMPA, 1500000)
    }),
    c(0x0403, 0xbdc8, "turtelizer", |device| {
        init_ftdi(device, &ftdi::devices::TURTELIZER, 1500000)
    }),
    c(0x03fd, 0x0008, "xpc", |device| {
        Box::pin(async { Ok(Box::new(xpc::Device::new(device).await?) as BoxedBackend) })
    }),
];
