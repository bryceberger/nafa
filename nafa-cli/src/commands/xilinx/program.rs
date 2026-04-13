use std::path::PathBuf;

use nafa_io::Controller;

#[derive(clap::Args)]
pub struct Args {
    pub input_file: PathBuf,
}

pub async fn run(
    cont: &mut Controller,
    pb: Option<&indicatif::ProgressBar>,
    args: Args,
) -> Result<Option<Box<dyn FnOnce()>>, eyre::Error> {
    let mut data = std::fs::read(&args.input_file)?;
    for d in &mut data {
        *d = d.reverse_bits();
    }
    if let Some(pb) = pb {
        pb.set_length(data.len() as _)
    }

    let stats = nafa_xilinx::_32bit::program(cont, &data).await?;

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
