use std::path::PathBuf;

use eyre::{OptionExt as _, Result};
use nafa_io::units::Bytes;
use nafa_xilinx::_32bit::{Controller, actions};

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
    let len = len.ok_or_eyre("unsupported device for readback")?;

    if let Some(pb) = pb {
        pb.set_length(Bytes::from(len).0 as _);
    }

    let data = actions::readback::run(cont, len.into()).await?;
    std::fs::write(args.output_file, data)?;
    Ok(None)
}
