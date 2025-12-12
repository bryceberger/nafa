use std::{
    collections::HashMap,
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use clap::Parser;
use color_eyre::Result;
use nafa_io::{
    Backend as BackendTrait, Command, Controller, ShortHex,
    devices::{DeviceInfo, IdCode},
    units::{Bytes, Words32},
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
        global = true,
    )]
    usb: UsbAddr,

    /// Disable the progress bar
    #[arg(long, global = true)]
    no_progress_bar: bool,
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

    // no controller
    if let CliCommand::FlashXpc(flash) = command {
        return smol::block_on(flash_xpc(global.usb, flash));
    }

    let mut cont = smol::block_on(get_controller(&get_devices(), global.usb))?;
    let progress = match command {
        CliCommand::Readback(_) | CliCommand::Program(_) => !global.no_progress_bar,
        _ => false,
    };
    if progress {
        let notify = &AtomicUsize::new(0);
        let done = &AtomicBool::new(false);
        let pb = &setup_progress_bar();
        std::thread::scope(|s| {
            s.spawn(move || {
                while !done.load(Ordering::Acquire) {
                    pb.set_position(notify.load(Ordering::Acquire) as _);
                }
            });

            let r = cont.with_notifications(notify, |cont| run(command, cont, Some(pb)));
            done.store(true, Ordering::Release);
            r
        })?;
    } else {
        run(command, &mut cont, None)?;
    }
    Ok(())
}

fn setup_progress_bar() -> indicatif::ProgressBar {
    let template =
        "{spinner:.green} {elapsed:>3}/{duration:>3} {bar} {bytes}/{total_bytes} ({bytes_per_sec})";
    let style = indicatif::ProgressStyle::with_template(template).unwrap();
    let pb = indicatif::ProgressBar::new(0)
        .with_finish(indicatif::ProgressFinish::Abandon)
        .with_style(style);
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

fn run(
    command: CliCommand,
    cont: &mut Controller<Box<dyn BackendTrait>>,
    pb: Option<&indicatif::ProgressBar>,
) -> Result<()> {
    match command {
        // no controller, handled earlier
        CliCommand::FlashXpc(_) => unreachable!(),

        // controller
        CliCommand::Info => {
            smol::block_on(info(cont))?;
        }
        CliCommand::InfoXadc => {
            smol::block_on(info_xadc(cont))?;
        }
        CliCommand::Readback(args) => {
            let data = match &cont.info().specific {
                nafa_io::devices::Specific::Unknown => todo!(),
                nafa_io::devices::Specific::Xilinx32(info) => {
                    let len = info.readback.into();
                    if let Some(pb) = pb {
                        pb.set_length(Bytes::from(info.readback).0 as _)
                    }
                    smol::block_on(nafa_xilinx::_32bit::readback(cont, len))?
                }
            };
            std::fs::write(args.output_file, data)?;
        }
        CliCommand::Program(args) => {
            let mut data = std::fs::read(args.input_file)?;
            for d in &mut data {
                *d = d.reverse_bits();
            }
            if let Some(pb) = pb {
                pb.set_length(data.len() as _)
            }
            smol::block_on(nafa_xilinx::_32bit::program(cont, &data))?;
        }
    }

    Ok(())
}

fn get_devices() -> HashMap<IdCode, DeviceInfo> {
    nafa_io::devices::builtin().collect()
}

async fn get_controller(
    devices: &HashMap<IdCode, DeviceInfo>,
    addr: UsbAddr,
) -> Result<Controller<Box<dyn BackendTrait>>> {
    let device = get_device(addr).await?;
    let mut backend = match nafa_io::cables::init(device).await {
        Ok(b) => b,
        Err(errs) => return Err(eyre::eyre!("{:?}", errs)),
    };
    let devices = nafa_io::detect_chain(&mut backend, devices).await?;
    let (before, device, after) = match &devices[..] {
        [] => return Err(eyre::eyre!("no devices detected on jtag chain")),
        [single] => (vec![], single.clone(), vec![]),
        multiple => {
            struct DisplayableInfo {
                idx: usize,
                idcode: u32,
                name: &'static str,
            }
            impl std::fmt::Display for DisplayableInfo {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{:08X} {}", self.idcode, self.name)
                }
            }
            let options = multiple.iter().enumerate();
            let options = options.map(|(idx, (idcode, info))| DisplayableInfo {
                idx,
                idcode: *idcode,
                name: info.name,
            });
            // TODO: options for selecting this other than inquire (like a flag)
            let DisplayableInfo { idx, .. } =
                inquire::Select::new("choose device", options.collect()).prompt()?;

            let collect =
                |items: &[(u32, DeviceInfo)]| items.iter().map(|(_, info)| info.clone()).collect();
            let before = collect(&multiple[..idx]);
            let chosen = multiple[idx].clone();
            let after = collect(&multiple[idx + 1..]);
            (before, chosen, after)
        }
    };
    Controller::new(backend, before, device, after).await
}

async fn get_device(addr: UsbAddr) -> Result<nusb::DeviceInfo> {
    let Some(device) = nusb::list_devices()
        .await?
        .find(|d| d.vendor_id() == addr.vid && d.product_id() == addr.pid)
    else {
        return Err(eyre::eyre!("failed to open device {addr}"));
    };
    Ok(device)
}

async fn flash_xpc(addr: UsbAddr, args: FlashXpc) -> Result<()> {
    let device = get_device(addr).await?.open().await?;
    let firmware = match args.firmware {
        Firmware::XP2 => xpc::firmware::XP2,
    };
    xpc::flash(&device, firmware).await?;
    Ok(())
}

async fn info<B: BackendTrait>(cont: &mut Controller<B>) -> Result<()> {
    use nafa_xilinx::_32bit::{
        commands::{FUSE_DNA, FUSE_KEY, FUSE_USER, FUSE_USER_128, IDCODE, USERCODE},
        registers::{Addr, OpCode, Type1},
    };

    // TODO: zynq needs this to be replicated, or it spits out garbage
    //
    // specifically:
    // strategy     idcode  efuse
    // noop low       ✕       ✕
    // noop high      ✕       ✓
    // bypass low     ✓       ✕
    // bypass high    ✕       ✓
    // replicated     ✓       ✓
    //
    // I _assume_ this has something to do with 6 of the bits being for the
    // processor and 6 for the FPGA. Need to check docs to figure out exact scheme
    // for that, and if there's any info about this.
    let x = |cmd: u8| u32::from(cmd) << 6 | u32::from(cmd);
    for (name, cmd, len) in [
        ("idcode", x(IDCODE), Bytes(4)),
        ("usercode", x(USERCODE), Bytes(4)),
        // TODO: length of fuse_dna is 64 bits on S7, 96 on US(+)
        ("fuse_dna", x(FUSE_DNA), Bytes(12)),
        ("fuse_key", x(FUSE_KEY), Bytes(32)),
        ("fuse_user", x(FUSE_USER), Bytes(4)),
        // TODO: this register is not available on S7
        ("fuse_user_128", x(FUSE_USER_128), Bytes(16)),
    ] {
        let data = cont.run([Command::ir(cmd), Command::dr_rx(len)]).await?;
        println!("{:>15}: {}", name, ShortHex(data));
    }

    let x = |addr| Type1::new(OpCode::Read, addr, Words32(1));
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
        let out = nafa_xilinx::_32bit::read_register(cont, reg).await?;
        println!("{name:>15}: {}", ShortHex(out));
    }
    Ok(())
}

async fn info_xadc<B: BackendTrait>(cont: &mut Controller<B>) -> Result<()> {
    use nafa_xilinx::_32bit::drp::{Addr, Cmd, Command};

    let family = match &cont.info.specific {
        nafa_io::devices::Specific::Unknown => todo!(),
        nafa_io::devices::Specific::Xilinx32(info) => info.family,
    };

    println!("idcode: {:04X}", cont.idcode);
    println!("  name: {}", cont.info.name);

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
    let xadc_regs = nafa_xilinx::_32bit::read_xadc(cont, regs).await?;

    let show = |name: &str, addr: Addr, val: u16, unit: &str| {
        use nafa_xilinx::_32bit::drp::Transfer;
        const PREC: usize = 3;
        match addr.transfer(family) {
            Transfer::None | Transfer::Unknown => println!("{name}: {val:04X}"),
            Transfer::Exactly(f) => println!("{name}: {val:04X} => {:.PREC$}{unit}", f(val)),
            Transfer::OneOf(many) => {
                let mut it = many.iter();
                if let Some(first) = it.next() {
                    println!("{name}: {val:04X} => {:.PREC$}{unit}", first(val));
                }
                for f in it {
                    println!(
                        "{:len$}       => {:.PREC$}{unit}",
                        "",
                        f(val),
                        len = name.len()
                    );
                }
            }
        }
    };

    if let [_, temp, vccint, vccaux, vpvn, vrefp, vrefn, vcc_bram] = xadc_regs.as_chunks().0 {
        fn x(x: &[u8; 4]) -> u16 {
            u32::from_le_bytes(*x) as u16
        }

        show("  temp", Addr::Temperature, x(temp), "F");
        show("vccint", Addr::VccInt, x(vccint), "V");
        show("vccaux", Addr::VccAux, x(vccaux), "V");
        show("  vpvn", Addr::VpVn, x(vpvn), "V");
        show(" vrefp", Addr::VRefP, x(vrefp), "V");
        show(" vrefn", Addr::VRefN, x(vrefn), "V");
        show("  bram", Addr::VccBram, x(vcc_bram), "V");
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
