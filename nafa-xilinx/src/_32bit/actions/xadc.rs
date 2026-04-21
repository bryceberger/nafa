use eyre::Result;
use nafa_io::{Command, units::Bytes};

use crate::_32bit::{
    Controller,
    commands::{self, master},
    drp,
};

pub async fn run(
    cont: Controller<'_>,
    regs: impl IntoIterator<Item = drp::Command>,
) -> Result<&[u8]> {
    let num_slr = cont.info().slr;
    let drp_commands: Vec<[u8; 4]> = regs
        .into_iter()
        .map(|c| c.to_bits().to_le_bytes())
        .collect();

    let start = [Command::ir(master(commands::SYSMON_DRP, num_slr))];
    let between = [Command::idle(Bytes(10))];
    let after = [Command::dr_rx(Bytes(4))];

    let drp_commands = drp_commands
        .iter()
        .flat_map(|c| std::iter::once(Command::dr_txrx(c)).chain(between));

    cont.consume()
        .run(start.into_iter().chain(drp_commands).chain(after))
        .await
}
