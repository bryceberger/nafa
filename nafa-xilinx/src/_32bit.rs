use eyre::Result;
use nafa_io::{
    Backend, Command, Controller,
    units::{Bytes, Words32},
};

use self::registers::{Addr, Type1};

pub mod commands;
pub mod drp;
pub mod info;
pub mod registers;

fn shift_for_slr(active_slr: u8, inst: u8) -> u32 {
    assert!(active_slr <= 4);
    const NOOPS: u32 = 0b_100100_100100_100100_100100_100100;
    // const NOOPS: u32 = u32::MAX;
    let inst = u32::from(inst & 0b111111);
    NOOPS & !(0b111111 << (active_slr * 6)) | inst << (active_slr * 6)
}

pub async fn read_device_register(
    cont: &mut Controller<impl Backend>,
    active_slr: u8,
    reg: Type1,
) -> Result<&[u8]> {
    let tiny_bitstream =
        bitstream_to_wire_order([Type1::SYNC, Type1::NOOP, reg.to_raw(), Type1::NOOP, Type1::NOOP]);
    let tiny_bitstream = tiny_bitstream.as_flattened();

    cont.run([
        Command::ir(shift_for_slr(active_slr, commands::CFG_IN)),
        Command::dr_tx(tiny_bitstream),
        Command::ir(shift_for_slr(active_slr, commands::CFG_OUT)),
        Command::dr_rx(Bytes::from(reg.word_count.into_())),
    ])
    .await
}

async fn read_device_register_word(
    cont: &mut Controller<impl Backend>,
    active_slr: u8,
    addr: Addr,
) -> Result<u32> {
    let data = read_device_register_sized(cont, active_slr, addr).await?;
    let data = data.map(|x| x.reverse_bits());
    Ok(u32::from_be_bytes(data))
}

async fn read_device_register_sized<const N: usize>(
    cont: &mut Controller<impl Backend>,
    active_slr: u8,
    addr: Addr,
) -> Result<&[u8; N]> {
    const { assert!(N.is_multiple_of(4)) };
    let n = u16::try_from(N / 4).unwrap();
    read_device_register(
        cont,
        active_slr,
        Type1::new(registers::OpCode::Read, addr, Words32(n)),
    )
    .await
    .map(|x| {
        x.try_into()
            .expect("dr_rx() should always return exact len")
    })
}

async fn read_jtag_register<B: Backend>(
    cont: &mut Controller<B>,
    active_slr: u8,
    inst: u8,
    len: Bytes<usize>,
) -> Result<&[u8]> {
    cont.run([Command::ir(shift_for_slr(active_slr, inst)), Command::dr_rx(len)])
        .await
}

async fn read_jtag_register_sized<const N: usize, B: Backend>(
    cont: &mut Controller<B>,
    active_slr: u8,
    inst: u8,
) -> Result<&[u8; N]> {
    read_jtag_register(cont, active_slr, inst, Bytes(N))
        .await
        .map(|x| {
            x.try_into()
                .expect("dr_rx() should always return exact len")
        })
}

pub async fn read_xadc<B: Backend>(
    cont: &mut Controller<B>,
    active_slr: u8,
    regs: impl IntoIterator<Item = drp::Command>,
) -> Result<&[u8]> {
    let drp_commands: Vec<[u8; 4]> = regs
        .into_iter()
        .map(|c| c.to_bits().to_le_bytes())
        .collect();

    let start = [Command::ir(shift_for_slr(active_slr, commands::XADC_DRP))];
    let between = [Command::idle(Bytes(10))];
    let after = [Command::dr_rx(Bytes(4))];

    let drp_commands = drp_commands
        .iter()
        .flat_map(|c| std::iter::once(Command::dr_txrx(c)).chain(between));

    cont.run(start.into_iter().chain(drp_commands).chain(after))
        .await
}

pub async fn program<B: Backend>(cont: &mut Controller<B>, data: &[u8]) -> Result<()> {
    cont.run([
        Command::ir(commands::JSHUTDOWN as _),
        Command::ir(commands::CFG_IN as _),
        Command::dr_tx_with_notification(data),
        Command::ir(commands::JSTART as _),
    ])
    .await?;
    Ok(())
}

pub async fn readback<B: Backend>(cont: &mut Controller<B>, len: Bytes<usize>) -> Result<&[u8]> {
    use self::registers::{Addr, OpCode, type2};
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
        Command::ir(commands::CFG_IN as _),
        Command::dr_tx(readback),
        Command::ir(commands::CFG_OUT as _),
        Command::dr_rx_with_notification(len),
    ];

    cont.run(commands).await
}

pub(crate) fn bitstream_to_wire_order<const N: usize>(x: [u32; N]) -> [[u8; 4]; N] {
    x.map(|x| x.to_be_bytes().map(u8::reverse_bits))
}
