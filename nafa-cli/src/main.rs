use std::{
    collections::HashMap,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

use clap::Parser;
use color_eyre::Result;
use nafa_io::{
    Backend, Controller, devices::DeviceInfo, jtag::IdCode, units::Bytes, usb_blaster, xpc,
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
#[command(next_help_heading = "Global Options")]
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
    DetectChain,
    Info {
        #[arg(short, long)]
        pretty: bool,
    },
    InfoXadc,
    Flash(Flash),
    Readback(Readback),
    Program(Program),
    ProgramBbramKey(ProgramBbramKey),
}

#[derive(clap::Args)]
struct Flash {
    firmware: Firmware,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum Firmware {
    XP2,
    UsbBlasterII,
}

#[derive(clap::Args)]
struct Readback {
    output_file: PathBuf,
}

#[derive(clap::Args)]
struct Program {
    input_file: PathBuf,
}

#[derive(clap::Args)]
struct ProgramBbramKey {
    #[command(flatten)]
    key_source: BbramKeySource,
    #[command(flatten)]
    dpa: Option<nafa_xilinx::_32bit::bbram::Dpa>,
}

#[derive(clap::Args)]
#[group(required = true, multiple = false)]
struct BbramKeySource {
    /// 32-byte hexadecimal value. Can be repeated for devices with multiple
    /// SLRs.
    #[arg(long)]
    key: Option<Vec<cli_helpers::Hex<32>>>,
    /// `.nky` file as used by Vivado.
    #[arg(long)]
    nky: Option<PathBuf>,
}

fn main() -> Result<()> {
    init_logging()?;
    let args = Args::parse();
    smol::block_on(async_main(args))
}

async fn async_main(Args { global, command }: Args) -> Result<()> {
    // no controller
    match command {
        CliCommand::Flash(flash) => return flash_xpc(global.usb, flash).await,
        CliCommand::DetectChain => {
            let backend = &mut get_backend(global.usb).await?;
            let chain = nafa_io::detect_chain(backend, &get_device_map()).await?;
            for (idx, (idcode, info)) in chain.iter().enumerate() {
                let code = idcode.code();
                let info = nafa_io::controller::IdCodeInfo::new(4, *idcode, Some(info));
                println!("{idx}: {code:08X}\n{info}");
            }
            return Ok(());
        }
        _ => (),
    }

    let mut cont = get_controller(&get_device_map(), global.usb, global.jtag_idx).await?;
    let progress = match command {
        CliCommand::Readback(_) | CliCommand::Program(_) => !global.no_progress_bar,
        _ => false,
    };
    let action = if progress {
        let notify = AtomicUsize::new(0);
        let pb = setup_progress_bar();
        let progress = smol::future::poll_fn(|_| {
            pb.set_position(notify.load(Ordering::Acquire) as _);
            std::task::Poll::Pending
        });
        cont.with_notifications(&notify, async |cont| run(command, cont, Some(&pb)).await)
            .race(progress)
            .await?
    } else {
        run(command, &mut cont, None).await?
    };
    if let Some(action) = action {
        action()
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
) -> Result<Option<Box<dyn FnOnce()>>> {
    match command {
        // no controller, handled earlier
        CliCommand::Flash(_) | CliCommand::DetectChain => unreachable!(),

        CliCommand::ProgramBbramKey(opts) => {
            program_bbram(cont, opts).await?;
        }
        // controller
        CliCommand::Info { pretty } => {
            info(cont, pretty).await?;
        }
        CliCommand::InfoXadc => {
            info_xadc(cont).await?;
        }
        CliCommand::Readback(args) => {
            use nafa_io::devices::Specific as S;
            let data = match &cont.info().specific {
                S::Unknown | S::Intel => todo!(),
                S::Xilinx32(info) => {
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

            pub const fn as_millis(d: std::time::Duration) -> f32 {
                const NANOS_PER_MILLI: u32 = 1_000_000;
                (d.as_nanos() as f32) / (NANOS_PER_MILLI as f32)
            }

            let stats = nafa_xilinx::_32bit::program(cont, &data).await?;
            return Ok(Some(Box::new(move || {
                println!("shutdown: {:>7.3}ms", as_millis(stats.time_shutdown));
                println!(" program: {:>7.3}ms", as_millis(stats.time_program));
                println!("  verify: {:>7.3}ms", as_millis(stats.time_verify));
                println!(" success: {}", stats.success);
            })));
        }
    }

    Ok(None)
}

async fn program_bbram(
    cont: &mut Controller<Box<dyn Backend + 'static>>,
    opts: ProgramBbramKey,
) -> Result<(), eyre::Error> {
    use nafa_io::devices::{Specific, Xilinx32Info};
    use nafa_xilinx::_32bit::{bbram, nky};
    let num_slr = match cont.info().specific {
        Specific::Xilinx32(Xilinx32Info { slr, .. }) => slr,
        _ => return Err(eyre::eyre!("can only program bbram for xilinx device")),
    };
    let keys = if let Some(path) = opts.key_source.nky {
        nky::Nky::parse(&smol::fs::read_to_string(path).await?)?.keys
    } else {
        let keys = opts.key_source.key.expect("clap validated");
        keys.into_iter().map(|x| x.0).collect()
    };
    if usize::from(num_slr) != keys.len() {
        return Err(eyre::eyre!(
            "device requires {} keys, {} provided",
            num_slr,
            keys.len()
        ));
    }
    bbram::program_key(cont, &keys, opts.dpa).await?;
    Ok(())
}

fn get_device_map() -> HashMap<IdCode, DeviceInfo> {
    nafa_io::devices::builtin().collect()
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

async fn get_backend(addr: UsbAddr) -> Result<Box<dyn Backend>, eyre::Error> {
    let device = get_device(addr).await?;
    match nafa_io::cables::init(device).await {
        Ok(b) => Ok(b),
        Err(errs) => Err(eyre::eyre!("failed to init cable: {errs:?}")),
    }
}

async fn get_controller(
    devices: &HashMap<IdCode, DeviceInfo>,
    addr: UsbAddr,
    jtag_idx: Option<usize>,
) -> Result<Controller<Box<dyn Backend>>> {
    fn chain_info(devices: &[(IdCode, DeviceInfo)]) -> String {
        let devices = devices.iter().enumerate();
        devices.fold(String::new(), |mut acc, (idx, (idcode, info))| {
            use std::fmt::Write;
            let code = idcode.code();
            let name = info.name;
            write!(&mut acc, "\n    {idx:>2}: {code:08X} {name}")
                .expect("write to string cannot fail");
            acc
        })
    }

    let mut backend = get_backend(addr).await?;

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
            let before = multiple[..idx].to_vec();
            let chosen = multiple[idx].clone();
            let after = multiple[idx + 1..].to_vec();
            (before, chosen, after)
        }
    };
    Controller::new(backend, before, device, after).await
}

async fn flash_xpc(addr: UsbAddr, args: Flash) -> Result<()> {
    let device = get_device(addr).await?.open().await?;
    let firmware = match args.firmware {
        Firmware::XP2 => xpc::firmware::XP2,
        Firmware::UsbBlasterII => usb_blaster::firmware::BLASTER_6810,
    };
    xpc::flash(&device, firmware).await?;
    Ok(())
}

async fn info(cont: &mut Controller<impl Backend>, pretty: bool) -> Result<()> {
    use facet_pretty::FacetPretty;

    fn print<'a, F: facet::Facet<'a>>(info: F, pretty: bool) -> Result<()> {
        if pretty {
            println!("{}", info.pretty());
        } else {
            facet_json::to_writer_std(std::io::stdout(), &info)?;
        }
        Ok(())
    }

    let info = nafa_xilinx::read(cont).await?;
    print(info, pretty)?;
    Ok(())
}

async fn info_xadc<B: Backend>(cont: &mut Controller<B>) -> Result<()> {
    use nafa_io::devices::Specific as S;
    use nafa_xilinx::_32bit::drp::{Addr, Cmd, Command};

    let family = match &cont.info().specific {
        S::Xilinx32(info) => info.family,
        _ => unreachable!(),
    };

    println!("idcode: {:04X}", cont.idcode().code());
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
