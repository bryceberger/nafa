use std::path::PathBuf;

use nafa_io::{Controller, devices::Specific, units::Bytes};

#[derive(clap::Args)]
pub struct Args {
    pub output_file: PathBuf,
}

pub async fn run(
    cont: &mut Controller,
    pb: Option<&indicatif::ProgressBar>,
    args: Args,
) -> Result<Option<Box<dyn FnOnce()>>, eyre::Error> {
    let data = match &cont.info().specific {
        Specific::Xilinx32(info) => {
            let len = info.readback.into();
            if let Some(pb) = pb {
                pb.set_length(Bytes::from(info.readback).0 as _)
            }
            nafa_xilinx::_32bit::readback(cont, len).await?
        }
        _ => todo!("called xilinx readback with non-xilinx active device"),
    };
    std::fs::write(args.output_file, data)?;
    Ok(None)
}
