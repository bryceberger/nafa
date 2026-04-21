use std::path::PathBuf;

use eyre::Result;
use nafa_xilinx::_16bit::{Controller, actions};

use crate::cli_helpers::as_millis;

#[derive(clap::Args)]
pub struct Args {
    input_file: PathBuf,
}

pub async fn run(
    cont: Controller<'_>,
    pb: Option<&indicatif::ProgressBar>,
    args: Args,
) -> Result<Option<Box<dyn FnOnce()>>> {
    let mut data = std::fs::read(&args.input_file)?;
    for d in &mut data {
        *d = d.reverse_bits();
    }
    if let Some(pb) = pb {
        pb.set_length(data.len() as _)
    }

    let stats = actions::program::run(cont, &data).await?;
    let digits = as_millis(stats.time_program)
        .max(as_millis(stats.time_verify))
        .log10()
        .ceil() as usize;
    let width = digits + 4;
    Ok(Some(Box::new(move || {
        println!("program: {:>width$.3}ms", as_millis(stats.time_program));
        println!(" verify: {:>width$.3}ms", as_millis(stats.time_verify));
        println!("success: {}", stats.success);
    })))
}
