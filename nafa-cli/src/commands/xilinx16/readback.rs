use std::path::PathBuf;

use eyre::Result;
use nafa_io::units::Bytes;
use nafa_xilinx::_16bit::{Controller, actions};

#[derive(clap::Args)]
pub struct Args {
    pub output_file: PathBuf,
}

pub async fn run(
    cont: Controller<'_>,
    pb: Option<&indicatif::ProgressBar>,
    args: Args,
) -> Result<Option<Box<dyn FnOnce()>>> {
    let len = cont.info().readback;

    if let Some(pb) = pb {
        pb.set_length(Bytes::from(len.map(|x: u32| x as usize)).0 as u64);
    }

    let data = actions::readback::run(cont).await?;
    std::fs::write(args.output_file, data)?;
    Ok(None)
}
