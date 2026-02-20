use std::{
    collections::HashMap,
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use clap::Parser;
use color_eyre::Result;
use nafa_io::{
    Backend, Controller,
    devices::{DeviceInfo, IdCode},
    units::Bytes,
    xpc,
};
use smol::future::FutureExt;

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

    /// Device to open if there are multiple devices on the JTAG chain.
    #[arg(long, global = true)]
    jtag_idx: Option<usize>,

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
    let args = Args::parse();
    smol::block_on(async_main(args))
}

async fn async_main(Args { global, command }: Args) -> Result<()> {
    // no controller
    if let CliCommand::FlashXpc(flash) = command {
        return flash_xpc(global.usb, flash).await;
    }

    let mut cont = get_controller(&get_devices(), global.usb, global.jtag_idx).await?;
    let progress = match command {
        CliCommand::Readback(_) | CliCommand::Program(_) => !global.no_progress_bar,
        _ => false,
    };
    if progress {
        let notify = &AtomicUsize::new(0);
        let done = &AtomicBool::new(false);
        let pb = &setup_progress_bar();

        let progress = async {
            while !done.load(Ordering::Acquire) {
                pb.set_position(notify.load(Ordering::Acquire) as _);
                smol::future::yield_now().await;
            }
            Ok(())
        };
        let runner = cont.with_notifications(notify, async |cont| {
            let r = run(command, cont, Some(pb)).await;
            done.store(true, Ordering::Release);
            r
        });
        runner.race(progress).await?;
    } else {
        run(command, &mut cont, None).await?;
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

async fn run(
    command: CliCommand,
    cont: &mut Controller<Box<dyn Backend>>,
    pb: Option<&indicatif::ProgressBar>,
) -> Result<()> {
    match command {
        // no controller, handled earlier
        CliCommand::FlashXpc(_) => unreachable!(),

        // controller
        CliCommand::Info => {
            info(cont).await?;
        }
        CliCommand::InfoXadc => {
            info_xadc(cont).await?;
        }
        CliCommand::Readback(args) => {
            let data = match &cont.info().specific {
                nafa_io::devices::Specific::Unknown => todo!(),
                nafa_io::devices::Specific::Xilinx32(info) => {
                    let len = info.readback.into();
                    if let Some(pb) = pb {
                        pb.set_length(Bytes::from(info.readback).0 as _)
                    }
                    nafa_xilinx::_32bit::readback(cont, len).await?
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
            nafa_xilinx::_32bit::program(cont, &data).await?;
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
    jtag_idx: Option<usize>,
) -> Result<Controller<Box<dyn Backend>>> {
    fn chain_info(devices: &[(u32, DeviceInfo)]) -> String {
        let devices = devices.iter().enumerate();
        devices.fold(String::new(), |mut acc, (idx, (idcode, info))| {
            use std::fmt::Write;
            write!(&mut acc, "\n    {idx:>2}: {idcode:08X} {}", info.name)
                .expect("write to string cannot fail");
            acc
        })
    }

    let device = get_device(addr).await?;
    let mut backend = match nafa_io::cables::init(device).await {
        Ok(b) => b,
        Err(errs) => return Err(eyre::eyre!("{:?}", errs)),
    };

    let devices = nafa_io::detect_chain(&mut backend, devices).await?;
    let (before, device, after) = match (&devices[..], jtag_idx) {
        ([], _) => return Err(eyre::eyre!("no devices detected on jtag chain")),

        ([single], Some(0) | None) => (vec![], single.clone(), vec![]),

        (multiple, Some(idx)) if idx >= multiple.len() => {
            return Err(eyre::eyre!(
                "idx {idx} too large for chain:{}",
                chain_info(multiple)
            ));
        }
        (multiple, None) => {
            return Err(eyre::eyre!(
                "multiple devices on jtag chain, but no index provided:{}",
                chain_info(multiple)
            ));
        }

        (multiple, Some(idx)) => {
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

async fn info(cont: &mut Controller<impl Backend>) -> Result<()> {
    use nafa_io::devices::{Specific as S, Xilinx32Family as F, Xilinx32Info as I};
    use nafa_xilinx::{_32bit::info as info_32, read, zynq_32::info as info_zynq};

    let stdout = std::io::stdout().lock();

    match cont.info().specific {
        S::Xilinx32(I { family: F::S7, .. }) => {
            let info: info_32::S7 = read(cont).await?;
            facet_json::to_writer_std(stdout, &info)?;
        }
        S::Xilinx32(I { family: F::US, .. }) => {
            let info: info_32::US = read(cont).await?;
            facet_json::to_writer_std(stdout, &info)?;
        }
        S::Xilinx32(I { family: F::UP, .. }) => {
            let info: info_32::UP = read(cont).await?;
            facet_json::to_writer_std(stdout, &info)?;
        }
        S::Xilinx32(I { family: F::ZP, .. }) => {
            let info: info_zynq::ZP = read(cont).await?;
            facet_json::to_writer_std(stdout, &info)?;
        }
        _ => return Err(eyre::eyre!("unsupported device")),
    }
    Ok(())
}

async fn info_xadc<B: Backend>(cont: &mut Controller<B>) -> Result<()> {
    use nafa_xilinx::_32bit::drp::{Addr, Cmd, Command};

    let family = match &cont.info().specific {
        nafa_io::devices::Specific::Unknown => todo!(),
        nafa_io::devices::Specific::Xilinx32(info) => info.family,
    };

    println!("idcode: {:04X}", cont.idcode());
    println!("  name: {}", cont.info().name);

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
    let xadc_regs = nafa_xilinx::_32bit::read_xadc(cont, 0, regs).await?;

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
    use tracing::{Level, Metadata};
    use tracing_subscriber::{EnvFilter, fmt, layer::Context, prelude::*};

    // `nusb` emits `log::error!()` calls for various things. However, we handle /
    // expect some errors. It's annoying to see 4 instances of "failed to claim
    // interface" during normal device discovery.
    struct NoNusbErrors;
    impl<S> tracing_subscriber::layer::Filter<S> for NoNusbErrors {
        fn enabled(&self, meta: &Metadata<'_>, _cx: &Context<'_, S>) -> bool {
            !(meta.target() == "nusb::error" && *meta.level() == Level::ERROR)
        }
    }

    tracing_subscriber::registry()
        .with(fmt::layer().with_filter(NoNusbErrors))
        .with(EnvFilter::from_default_env())
        .with(tracing_error::ErrorLayer::default())
        .init();
    color_eyre::install()?;
    Ok(())
}
