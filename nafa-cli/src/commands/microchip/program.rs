use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use eyre::Result;
use nafa_io::{Command, Controller, units::Bytes};
use nafa_microchip::{_32bit, read};

pub struct ProgramStats {
    pub time_shutdown: Duration,
    pub time_program: Duration,
    pub time_verify: Duration,
    pub success: bool,
}

#[derive(clap::Args)]
pub struct Args {
    pub input_file: PathBuf,
}

pub async fn run(
    cont: &mut Controller,
    pb: Option<&indicatif::ProgressBar>,
    args: Args,
) -> Result<Option<Box<dyn FnOnce()>>> {
    let data = std::fs::read(&args.input_file)?;
    if let Some(pb) = pb {
        pb.set_length(data.len() as _)
    }

    let stats = _32bit::program::run(cont, &data).await?;

    let digits = as_millis(stats.time_program)
        .max(as_millis(stats.time_shutdown))
        .max(as_millis(stats.time_verify))
        .log10()
        .ceil() as usize;
    let width = digits + 4;
    Ok(Some(Box::new(move || {
        println!("shutdown: {:>width$.3}ms", as_millis(stats.time_shutdown));
        println!(" program: {:>width$.3}ms", as_millis(stats.time_program));
        println!("  verify: {:>width$.3}ms", as_millis(stats.time_verify));
        println!(" success: {}", stats.success);
    })))
}

fn as_millis(d: std::time::Duration) -> f32 {
    const NANOS_PER_MILLI: u32 = 1_000_000;
    (d.as_nanos() as f32) / (NANOS_PER_MILLI as f32)
}
