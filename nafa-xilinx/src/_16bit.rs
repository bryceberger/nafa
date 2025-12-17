use eyre::Result;
use nafa_io::{Backend, Command, Controller, units::Bytes};

use self::registers::Type1;

mod commands;
mod info;
mod registers;

pub fn read_register<B: Backend>(cont: &mut Controller<B>, reg: Type1) -> Result<&[u8]> {
    let tiny_bitstream = bitstream_to_wire_order([Type1::SYNC, Type1::NOOP, reg.to_raw()]);
    let tiny_bitstream = tiny_bitstream.as_flattened();

    cont.run([
        Command::ir(commands::CFG_IN as _),
        Command::dr_tx(tiny_bitstream),
        Command::ir(commands::CFG_OUT as _),
        Command::dr_rx(Bytes::from(reg.word_count.map(|x| x.into()))),
    ])
}

fn bitstream_to_wire_order<const N: usize>(x: [u16; N]) -> [[u8; 2]; N] {
    x.map(|x| x.to_be_bytes().map(u8::reverse_bits))
}
