use eyre::Result;
use nafa_io::{Command, units::Words16};

use crate::_16bit::{
    Controller, bitstream_to_wire_order, commands, from_wire_order,
    registers::{self, Addr, OpCode},
};

pub async fn run(mut cont: Controller<'_>) -> Result<u16> {
    let bitstream = bitstream_to_wire_order([
        registers::SYNC0,
        registers::SYNC1,
        registers::NOOP,
        registers::type1(OpCode::Read, Addr::Stat, Words16(1)),
        registers::NOOP,
        registers::NOOP,
    ]);
    let commands = [
        Command::ir(commands::CFG_IN),
        Command::dr_tx(bitstream.as_flattened()),
        Command::ir(commands::CFG_OUT),
        Command::dr_rx(Words16(1).into()),
    ];
    let data = cont.borrow().run(commands).await?;
    Ok(from_wire_order(data.try_into()?))
}
