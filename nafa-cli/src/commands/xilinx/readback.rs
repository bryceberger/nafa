use std::path::PathBuf;

use nafa_io::{Backend, Controller, devices::Specific, units::Bytes};

#[derive(clap::Args)]
pub struct Args {
    pub output_file: PathBuf,
}

pub async fn run(
    cont: &mut Controller<Box<dyn Backend>>,
    pb: Option<&indicatif::ProgressBar>,
    args: Args,
) -> Result<Option<Box<dyn FnOnce()>>, eyre::Error> {
    let data = match &cont.info().specific {
        Specific::Unknown | Specific::Intel => todo!(),
        Specific::Xilinx32(info) => {
            let len = info.readback.into();
            if let Some(pb) = pb {
                pb.set_length(Bytes::from(info.readback).0 as _)
            }
            nafa_xilinx::_32bit::readback(cont, len).await?
        }
    };
    std::fs::write(args.output_file, data)?;
    Ok(None)
}
