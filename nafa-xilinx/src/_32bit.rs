use eyre::Result;
use nafa_io::{
    Backend, Command, Controller,
    units::{Bytes, Words32},
};

pub mod commands;
pub mod drp;
pub mod registers;

use self::registers::Type1;

pub fn read_register<B: Backend>(cont: &mut Controller<B>, reg: Type1) -> Result<&[u8]> {
    let tiny_bitstream = bitstream_to_wire_order([Type1::SYNC, Type1::NOOP, reg.to_raw()]);
    let tiny_bitstream = tiny_bitstream.as_flattened();

    cont.run([
        Command::ir(commands::CFG_IN as _),
        Command::dr_tx(tiny_bitstream),
        Command::ir(commands::CFG_OUT as _),
        Command::dr_rx(Bytes(4)),
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

    let start = [Command::ir(commands::XADC_DRP as _)];
    let between = [Command::idle(Bytes(10))];
    let after = [Command::dr_rx(Bytes(4))];

    let drp_commands = drp_commands
        .iter()
        .flat_map(|c| std::iter::once(Command::dr_txrx(c)).chain(between));

    cont.run(start.into_iter().chain(drp_commands).chain(after))
}

pub fn program<B: Backend + Send>(cont: &mut Controller<B>, data: &[u8]) -> Result<()> {
    cont.run([Command::ir(commands::CFG_IN as _), Command::dr_tx(data)])?;
    Ok(())
}

pub fn readback<B: Backend + Send>(cont: &mut Controller<B>, len: Bytes<usize>) -> Result<&[u8]> {
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
        Command::ir(commands::CFG_IN as _),
        Command::dr_tx(readback),
        Command::ir(commands::CFG_OUT as _),
        Command::dr_rx(len),
    ];

    cont.run(commands)
}

fn bitstream_to_wire_order<const N: usize>(x: [u32; N]) -> [[u8; 4]; N] {
    x.map(|x| x.to_be_bytes().map(u8::reverse_bits))
}
