use std::{
    collections::HashMap,
    sync::atomic::{AtomicUsize, Ordering},
};

use clap::Parser;
use color_eyre::Result;
use nafa_io::{Backend, Controller, devices::DeviceInfo, jtag::IdCode};
use smol::future::FutureExt;

use crate::cli_helpers::UsbAddr;

mod cli_helpers;
mod commands;

#[derive(clap::Parser)]
struct Args {
    #[command(flatten)]
    global: Global,
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Args)]
#[command(next_help_heading = "Global Options")]
struct Global {
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
enum Command {
    #[command(flatten)]
    Standalone(StandaloneCommand),
    #[command(flatten)]
    Controller(ControllerCommand),
}

#[derive(clap::Subcommand)]
enum StandaloneCommand {
    DetectChain,
    Flash(commands::flash::Args),
}

#[derive(clap::Subcommand)]
enum ControllerCommand {
    #[command(subcommand)]
    Xilinx(commands::xilinx::Command),
}

impl ControllerCommand {
    fn wants_progress(&self) -> bool {
        match self {
            Self::Xilinx(command) => command.wants_progress(),
        }
    }
}

fn main() -> Result<()> {
    init_logging()?;
    let args = Args::parse();
    smol::block_on(async_main(args))
}

async fn async_main(Args { global, command }: Args) -> Result<()> {
    // no controller
    let command = match command {
        Command::Standalone(StandaloneCommand::DetectChain) => {
            let backend = &mut get_backend(global.usb).await?;
            let chain = nafa_io::detect_chain(backend, &get_device_map()).await?;
            for (idx, (idcode, info)) in chain.iter().enumerate() {
                let code = idcode.code();
                let info = nafa_io::controller::IdCodeInfo::new(4, *idcode, Some(info));
                println!("{idx}: {code:08X}\n{info}");
            }
            return Ok(());
        }
        Command::Standalone(StandaloneCommand::Flash(args)) => {
            return commands::flash::run(global.usb, args).await;
        }
        Command::Controller(c) => c,
    };

    let mut cont = get_controller(&get_device_map(), global.usb, global.jtag_idx).await?;
    let progress = !global.no_progress_bar && command.wants_progress();
    let action = if progress {
        let notify = AtomicUsize::new(0);
        let pb = setup_progress_bar();
        let progress = smol::future::poll_fn(|_| {
            pb.set_position(notify.load(Ordering::Acquire) as _);
            std::task::Poll::Pending
        });
        cont.with_notifications(&notify, async |cont| run(cont, Some(&pb), command).await)
            .race(progress)
            .await?
    } else {
        run(&mut cont, None, command).await?
    };
    if let Some(action) = action {
        action()
    }
    Ok(())
}

async fn run(
    cont: &mut Controller<Box<dyn Backend>>,
    pb: Option<&indicatif::ProgressBar>,
    command: ControllerCommand,
) -> Result<Option<Box<dyn FnOnce()>>, eyre::Error> {
    match command {
        ControllerCommand::Xilinx(cmd) => commands::xilinx::run(cont, pb, cmd).await,
    }
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
