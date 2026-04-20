use std::path::PathBuf;

use eyre::Result;
use nafa_xilinx::_32bit::{bbram, nky};
use nafa_xilinx::Controller;

#[derive(clap::Args)]
#[group(required = true, multiple = false)]
pub struct BbramKeySource {
    /// 32-byte hexadecimal value. Can be repeated for devices with multiple
    /// SLRs.
    #[arg(long)]
    pub key: Option<Vec<crate::cli_helpers::Hex<32>>>,
    /// `.nky` file as used by Vivado.
    #[arg(long)]
    pub nky: Option<PathBuf>,
}

#[derive(clap::Args)]
pub struct Args {
    #[command(flatten)]
    pub key_source: BbramKeySource,
    #[command(flatten)]
    pub dpa: Option<nafa_xilinx::_32bit::bbram::Dpa>,
}

pub async fn run(cont: Controller<'_>, opts: Args) -> Result<()> {
    let num_slr = cont.info().slr;
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
