use eyre::Result;
use nafa_io::{Command, Controller, ShortHex, units::Bytes};

#[derive(clap::Args)]
pub struct Args {
    #[arg(short, long)]
    pub pretty: bool,
}

pub async fn run(cont: &mut Controller, _: Args) -> Result<()> {
    let data = cont
        .run([Command::ir(0x0F), Command::dr_rx(Bytes(4))])
        .await?;
    println!("{data:02X?}");
    println!("{}", ShortHex(data));
    Ok(())
}
