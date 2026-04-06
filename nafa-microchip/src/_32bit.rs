use eyre::Result;
use nafa_io::{Command, Controller, units::Bytes};

pub mod commands;
pub mod info;

async fn read_jtag<const N: usize>(cont: &mut Controller, inst: commands::Cmd) -> Result<&[u8; N]> {
    let slice = cont
        .run([Command::ir(inst as u32), Command::dr_rx(Bytes(4))])
        .await?;
    Ok(slice.try_into()?)
}
