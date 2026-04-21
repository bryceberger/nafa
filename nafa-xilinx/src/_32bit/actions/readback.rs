use eyre::Result;
use nafa_io::{
    Command,
    units::{Bytes, Words32},
};

use crate::_32bit::{
    Controller, bitstream_to_wire_order,
    commands::{self, shifted},
    registers::{Addr, OpCode, Type1, type2},
};

pub async fn run(cont: Controller<'_>, len: Bytes<usize>) -> Result<&[u8]> {
    let num_slr = cont.info().slr;

    let readback = [
        Type1::SYNC,
        Type1::NOOP,
        Type1::new(OpCode::Write, Addr::Cmd, Words32(1)).to_raw(),
        0x0000_0004, // rcfg
        Type1::new(OpCode::Write, Addr::Far, Words32(1)).to_raw(),
        0x0000_0000,
        Type1::new(OpCode::Read, Addr::Fdro, Words32(0)).to_raw(),
        type2(OpCode::Read, 0xffffff),
        Type1::NOOP,
        Type1::NOOP,
    ];
    let readback = bitstream_to_wire_order(readback);
    let readback = readback.as_flattened();
    // so: the fdro read len
    // You would _think_ that this should be `args.len`, or maybe `args.len * 4` or
    // `* 32` because it's words or bytes or bits or something.
    //
    // HOWEVER, I cannot get any sensible value to work. Previous implementation has
    // a hardcoded value (0x5cdb57, reversed -> 0x3adbea). Neither the value nor
    // the reversed value is an even multiple of readback length for a basys
    // (548003 = 0x85ca3). Regardless, readback still works.
    //
    // It appears that you can just put in an arbitrarily high number, then just
    // request the _actual_ correct number of bytes with your `Command::RxBytes`.
    //
    // Notably, this does _not_ mess up subsequent `cont.run()`. If I were to guess,
    // going out of the `DR` side of JTAG makes the fpga just drop all further data.

    let commands = [
        Command::ir(shifted(commands::CFG_IN, num_slr, 0)),
        Command::dr_tx(readback),
        Command::ir(shifted(commands::CFG_OUT, num_slr, 0)),
        Command::dr_rx_with_notification(len),
    ];

    cont.consume().run(commands).await
}
