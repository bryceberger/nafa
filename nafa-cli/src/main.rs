#![feature(iter_intersperse)]

use std::{collections::HashMap, path::PathBuf};

use clap::Parser;
use color_eyre::{Result, eyre::eyre};
use nafa_io::{
    Backend as BackendTrait, Command, Controller, ShortHex,
    devices::DeviceInfo,
    ftdi::{self, devices},
    xpc,
};

use crate::cli_helpers::UsbAddr;

mod cli_helpers;

#[derive(clap::Parser)]
struct Args {
    #[command(flatten)]
    global: GlobalOpts,
    #[command(subcommand)]
    command: CliCommand,
}

#[derive(clap::Args)]
struct GlobalOpts {
    #[arg(
        long,
        default_value_t = UsbAddr { vid: 0x0403, pid: 0x6010 },
    )]
    usb: UsbAddr,

    #[arg(long, default_value = "ftdi")]
    backend: Backend,

    /// Disable the progress bar
    #[arg(long)]
    no_progress_bar: bool,
}

#[derive(Clone, Copy, Default, clap::ValueEnum)]
enum Backend {
    #[default]
    Ftdi,
    Xpc,
}

#[derive(clap::Subcommand)]
enum CliCommand {
    Info,
    InfoXadc,
    FlashXpc(FlashXpc),
    Readback(Readback),
    Program(Program),
}

#[derive(clap::Args)]
struct FlashXpc {
    firmware: Firmware,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Firmware {
    XP2,
}

#[derive(clap::Args)]
struct Readback {
    output_file: PathBuf,
}

#[derive(clap::Args)]
struct Program {
    input_file: PathBuf,
}

fn main() -> Result<()> {
    init_logging()?;
    let Args { global, command } = Args::parse();
    match command {
        // no controller
        CliCommand::FlashXpc(flash) => {
            flash_xpc(global.usb, flash)?;
        }

        // controller
        CliCommand::Info => {
            let mut cont = get_controller(global.backend, &get_devices(), global.usb)?;
            info(&mut cont)?;
        }
        CliCommand::InfoXadc => {
            let mut cont = get_controller(global.backend, &get_devices(), global.usb)?;
            info_xadc(&mut cont)?;
        }
        CliCommand::Readback(args) => {
            let mut cont = get_controller(global.backend, &get_devices(), global.usb)?;
            let data = match &cont.info().specific {
                nafa_io::devices::Specific::Unknown => todo!(),
                nafa_io::devices::Specific::Xilinx32(info) => {
                    let len = info.readback.into();
                    nafa_xilinx::_32bit::readback(&mut cont, len)?
                }
            };
            std::fs::write(args.output_file, data)?;
        }
        CliCommand::Program(args) => {
            let mut cont = get_controller(global.backend, &get_devices(), global.usb)?;
            let mut data = std::fs::read(args.input_file)?;
            for d in &mut data {
                *d = d.reverse_bits();
            }
            nafa_xilinx::_32bit::program(&mut cont, &data)?;
        }
    }
    Ok(())
}

fn get_devices() -> HashMap<u32, DeviceInfo> {
    nafa_io::devices::builtin().collect()
}

fn get_controller(
    backend: Backend,
    devices: &HashMap<u32, DeviceInfo>,
    addr: UsbAddr,
) -> Result<Controller<Box<dyn BackendTrait + Send>>> {
    let backend: Box<dyn BackendTrait + Send> = match backend {
        Backend::Ftdi => Box::new(get_device_ftdi(addr)?),
        Backend::Xpc => Box::new(get_device_xpc(addr)?),
    };
    Controller::new(backend, devices)
}

fn flash_xpc(addr: UsbAddr, args: FlashXpc) -> Result<()> {
    let device = rusb::open_device_with_vid_pid(addr.vid, addr.pid)
        .ok_or_else(|| eyre!("failed to open device {:04X}:{:04X}", addr.vid, addr.pid))?;
    let firmware = match args.firmware {
        Firmware::XP2 => xpc::firmware::XP2,
    };
    xpc::flash(&device, firmware)?;
    Ok(())
}

fn get_device_ftdi(addr: UsbAddr) -> Result<ftdi::Device> {
    let dev = ::ftdi::find_by_vid_pid(addr.vid, addr.pid).open()?;
    ftdi::Device::new(dev, &devices::NEXSYS4)
}

fn get_device_xpc(addr: UsbAddr) -> Result<xpc::Device> {
    let dev = rusb::open_device_with_vid_pid(addr.vid, addr.pid)
        .ok_or_else(|| eyre!("failed to open device {addr}"))?;
    xpc::Device::new(dev)
}

fn info<B: BackendTrait>(cont: &mut Controller<B>) -> Result<()> {
    use nafa_xilinx::_32bit::registers::{Addr, OpCode, Type1};
    for (name, cmd) in [
        ("idcode", nafa_xilinx::_32bit::commands::IDCODE),
        ("fuse_key", nafa_xilinx::_32bit::commands::FUSE_KEY),
        ("fuse_dna", nafa_xilinx::_32bit::commands::FUSE_DNA),
    ] {
        let data = cont.run([Command::ir(cmd.val as _), Command::dr_rx(cmd.read_len)])?;
        println!("{:>12}: {}", name, ShortHex(data));
    }
    let x = |addr| Type1::new(OpCode::Read, addr, 1);
    let regs = [
        ("boot_status", x(Addr::Bootsts)),
        ("cfg_status", x(Addr::Stat)),
        ("bspi", x(Addr::Bspi)),
        ("cor0", x(Addr::Cor0)),
        ("cor1", x(Addr::Cor1)),
        ("ctl0", x(Addr::Ctl0)),
        ("ctl1", x(Addr::Ctl1)),
    ];
    for (name, reg) in regs {
        let out = nafa_xilinx::_32bit::read_register(cont, reg)?;
        let data = u32::from_le_bytes(out.try_into().unwrap());
        println!("{name:>12}: {:08X} ({:08X})", data, data.reverse_bits());
    }
    Ok(())
}

fn info_xadc<B: BackendTrait>(cont: &mut Controller<B>) -> Result<()> {
    use nafa_xilinx::_32bit::drp::{
        Addr, Cmd, Command, adc_bipolar, adc_unipolar, power_supply, temperature,
    };
    let c = |addr| Command {
        cmd: Cmd::Read,
        addr,
        data: 0,
    };

    let regs = [
        c(Addr::Temperature),
        c(Addr::VccInt),
        c(Addr::VccAux),
        c(Addr::VpVn),
        c(Addr::VRefP),
        c(Addr::VRefN),
        c(Addr::VccBram),
    ];
    let xadc_regs = nafa_xilinx::_32bit::read_xadc(cont, regs)?;

    if let [_, temp, vccint, vccaux, vpvn, vrefp, vrefn, vcc_bram] = xadc_regs.as_chunks().0 {
        fn x(x: &[u8; 4]) -> u16 {
            u32::from_le_bytes(*x) as u16
        }
        fn c_to_f(c: f32) -> f32 {
            (c * 9. / 5.) + 32.
        }

        let temp_c = temperature(x(temp));
        let temp_f = c_to_f(temp_c);
        println!("  temp: {:04X} => {temp_c:.2}C, {temp_f:.2}F", x(temp));

        let vccint_ = power_supply(x(vccint));
        let vccaux_ = power_supply(x(vccaux));
        println!("vccint: {:04X} => {vccint_:.3}V", x(vccint));
        println!("vccaux: {:04X} => {vccaux_:.3}V", x(vccaux));

        let vpvn_u = adc_unipolar(x(vpvn));
        let vpvn_b = adc_bipolar(x(vpvn));
        println!("  vpvn: {:04X} => {vpvn_u:.3}V ({vpvn_b:.3}V)", x(vpvn));

        let vrefp_ = power_supply(x(vrefp));
        let vrefn_ = power_supply(x(vrefn));
        println!(" vrefp: {:04X} => {vrefp_:.3}V", x(vrefp));
        println!(" vrefn: {:04X} => {vrefn_:.3}V", x(vrefn));

        let vcc_bram_ = power_supply(x(vcc_bram));
        println!("  bram: {:04X} => {vcc_bram_:.3}V", x(vcc_bram));
    }

    Ok(())
}

fn init_logging() -> Result<()> {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .with(tracing_error::ErrorLayer::default())
        .init();
    color_eyre::install()?;
    Ok(())
}
