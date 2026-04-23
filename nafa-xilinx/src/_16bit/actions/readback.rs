use eyre::Result;
use nafa_io::{
    Command,
    units::{Bytes, Words16},
};

use crate::_16bit::{
    Controller, bitstream_to_wire_order, commands,
    registers::{self, Addr, OpCode},
};

pub async fn run(cont: Controller<'_>) -> Result<&[u8]> {
    let len = cont.info().readback;
    let readback = bitstream_to_wire_order([
        registers::SYNC0,
        registers::SYNC1,
        registers::NOOP,
        registers::type1(OpCode::Write, Addr::Cmd, Words16(1)),
        0b00100, // rcfg
        registers::type2(OpCode::Read, Addr::Fdro),
        0x0fff,
        0xffff,
        registers::NOOP,
        registers::NOOP,
    ]);
    let readback = readback.as_flattened();

    let commands = [
        Command::ir(commands::CFG_IN),
        Command::dr_tx(readback),
        Command::ir(commands::CFG_OUT),
        Command::dr_rx_with_notification(Bytes::from(len.map(|x: u32| x as usize))),
    ];
    cont.consume().run(commands).await
}
