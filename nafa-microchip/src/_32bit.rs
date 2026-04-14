use eyre::Result;
use nafa_io::{Command, Controller, units::Bytes};

pub mod commands;
pub mod info;

async fn read_jtag_simple<const N: usize>(cont: &mut Controller, inst: commands::Cmd, bytes: usize) -> Result<&[u8; N]> {
    let slice = cont
        .run([Command::ir(inst as u32), Command::dr_rx(Bytes(bytes))])
        .await?;
    Ok(slice.try_into()?)
}

async fn test_dsn<const N: usize>(cont: &mut Controller, inst: commands::Cmd, bytes: usize) -> Result<&[u8; N]> {
    let slice = cont
        .run([Command::ir(inst as u32), Command::dr_rx(Bytes(bytes)), Command::ir(inst as u32), Command::dr_rx(Bytes(bytes))])
        .await?;
    Ok(slice.try_into()?)
}
