use eyre::Result;
use nafa_io::{Command, Controller, ShortHex, units::Bytes};

#[derive(clap::Args)]
pub struct Args {
    #[arg(short, long)]
    pub pretty: bool,
}

pub async fn run(cont: &mut Controller, _: Args) -> Result<()> {
    
    todo!()
}
