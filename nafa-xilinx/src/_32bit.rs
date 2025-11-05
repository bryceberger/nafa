use eyre::Result;
use nafa_io::{
    Backend, Command, Controller,
    jtag::State,
    units::{Bits, Bytes, Words32},
};

pub mod commands;
pub mod drp;
pub mod registers;

use self::registers::Type1;

pub fn read_register<B: Backend>(cont: &mut Controller<B>, reg: Type1) -> Result<&[u8]> {
    let tiny_bitstream = bitstream_to_wire_order([Type1::SYNC, Type1::NOOP, reg.to_raw()]);
    let tiny_bitstream = tiny_bitstream.as_flattened();

    cont.run([
        Command::set_state(State::ShiftIR),
        Command::tx_bits(commands::CFG_IN, Bits(6)),
        Command::set_state(State::ShiftDR),
        Command::tx_bytes(tiny_bitstream),
        Command::set_state(State::ShiftIR),
        Command::tx_bits(commands::CFG_OUT, Bits(6)),
        Command::set_state(State::ShiftDR),
        Command::rx_bytes(Bytes(4)),
    ])
}

pub fn read_xadc<B: Backend>(
    cont: &mut Controller<B>,
    regs: impl IntoIterator<Item = drp::Command>,
) -> Result<&[u8]> {
    let drp_commands: Vec<[u8; 4]> = regs
        .into_iter()
        .map(|c| c.to_bits().to_le_bytes())
        .collect();

    let start = [
        Command::set_state(State::ShiftIR),
        Command::tx_bits(commands::XADC_DRP, Bits(6)),
        Command::set_state(State::ShiftDR),
    ];
    let between = [
        Command::set_state(State::RunTestIdle),
        Command::tx_bytes(&[0; 10]),
        Command::set_state(State::ShiftDR),
    ];
    let after = [Command::rx_bytes(Bytes(4))];

    let drp_commands = drp_commands
        .iter()
        .flat_map(|c| std::iter::once(Command::tx_rx_bytes(c)).chain(between));

    cont.run(start.into_iter().chain(drp_commands).chain(after))
}

pub fn program<B: Backend + Send>(cont: &mut Controller<B>, data: &[u8]) -> Result<()> {
    let commands = [
        Command::set_state(State::ShiftIR),
        Command::tx_bits(commands::CFG_IN, Bits(6)),
        Command::set_state(State::ShiftDR),
        Command::tx_bytes(data),
    ];

    cont.run(commands)?;
    Ok(())
}

pub fn readback<B: Backend + Send>(cont: &mut Controller<B>, len: Words32<usize>) -> Result<&[u8]> {
    use self::registers::{Addr, OpCode, type2};
    let readback = [
        Type1::SYNC,
        Type1::NOOP,
        Type1::new(OpCode::Write, Addr::Cmd, 1).to_raw(),
        0x0000_0004, // rcfg
        Type1::new(OpCode::Write, Addr::Far, 1).to_raw(),
        0x0000_0000,
        Type1::new(OpCode::Read, Addr::Fdro, 0).to_raw(),
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
        Command::set_state(State::ShiftIR),
        Command::tx_bits(commands::CFG_IN, Bits(6)),
        Command::set_state(State::ShiftDR),
        Command::tx_bytes(readback),
        Command::set_state(State::ShiftIR),
        Command::tx_bits(commands::CFG_OUT, Bits(6)),
        Command::set_state(State::ShiftDR),
        Command::rx_bytes(len.into()),
    ];

    cont.run(commands)
}

fn bitstream_to_wire_order<const N: usize>(x: [u32; N]) -> [[u8; 4]; N] {
    x.map(|x| x.to_be_bytes().map(u8::reverse_bits))
}
