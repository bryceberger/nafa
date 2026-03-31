use nafa_io::{usb_blaster, xpc};

use crate::cli_helpers::UsbAddr;

#[derive(clap::Args)]
pub struct Args {
    firmware: Firmware,
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum Firmware {
    XP2,
    UsbBlasterII,
}

pub async fn run(usb: UsbAddr, args: Args) -> Result<(), eyre::Error> {
    let device = nusb::list_devices()
        .await?
        .find(|d| d.vendor_id() == usb.vid && d.product_id() == usb.pid)
        .ok_or_else(|| eyre::eyre!("failed to open device {usb}"))?
        .open()
        .await?;

    let firmware = match args.firmware {
        Firmware::XP2 => xpc::firmware::XP2,
        Firmware::UsbBlasterII => usb_blaster::firmware::BLASTER_6810,
    };
    xpc::flash(&device, firmware).await?;
    Ok(())
}
