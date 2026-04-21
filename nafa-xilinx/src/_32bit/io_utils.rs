use eyre::Result;
use nafa_io::{
    Command,
    units::{Bytes, Words32},
};

use super::{
    Controller, bitstream_to_wire_order,
    commands::{self, shifted},
    from_wire_order,
    registers::{Addr, OpCode, Type1},
};
use crate::_32bit::commands::{duplicated, master};

pub async fn read_device_register<'a>(
    cont: Controller<'a>,
    active_slr: u8,
    reg: Type1,
) -> Result<&'a [u8]> {
    let tiny_bitstream =
        bitstream_to_wire_order([Type1::SYNC, Type1::NOOP, reg.to_raw(), Type1::NOOP, Type1::NOOP]);
    let tiny_bitstream = tiny_bitstream.as_flattened();
    let num_slr = cont.info().slr;

    cont.consume()
        .run([
            Command::ir(shifted(commands::CFG_IN, num_slr, active_slr)),
            Command::dr_tx(tiny_bitstream),
            Command::ir(shifted(commands::CFG_OUT, num_slr, active_slr)),
            Command::dr_rx(Bytes::from(reg.word_count.into_())),
        ])
        .await
}

pub async fn read_device_register_word(
    cont: Controller<'_>,
    active_slr: u8,
    addr: Addr,
) -> Result<u32> {
    let data = read_device_register_sized(cont, active_slr, addr).await?;
    let data = from_wire_order(*data);
    Ok(data)
}

async fn read_device_register_sized<const N: usize>(
    cont: Controller<'_>,
    active_slr: u8,
    addr: Addr,
) -> Result<&[u8; N]> {
    const { assert!(N.is_multiple_of(4)) };
    let n = u16::try_from(N / 4).unwrap();
    let slice =
        read_device_register(cont, active_slr, Type1::new(OpCode::Read, addr, Words32(n))).await?;
    Ok(slice.try_into()?)
}

pub async fn read_jtag_register_duplicated<const N: usize>(
    cont: Controller<'_>,
    inst: commands::Duplicated,
) -> Result<&[u8; N]> {
    let slice = cont
        .consume()
        .run([Command::ir(duplicated(inst)), Command::dr_rx(Bytes(N))])
        .await?;
    Ok(slice.try_into()?)
}

pub async fn read_jtag_register_master<const N: usize>(
    cont: Controller<'_>,
    inst: commands::Master,
) -> Result<&[u8; N]> {
    let num_slr = cont.info().slr;
    let slice = cont
        .consume()
        .run([Command::ir(master(inst, num_slr)), Command::dr_rx(Bytes(N))])
        .await?;
    Ok(slice.try_into()?)
}

pub async fn read_jtag_register_shifted<const N: usize>(
    cont: Controller<'_>,
    active_slr: u8,
    inst: commands::Shifted,
) -> Result<&[u8; N]> {
    let num_slr = cont.info().slr;
    let slice = cont
        .consume()
        .run([Command::ir(shifted(inst, num_slr, active_slr)), Command::dr_rx(Bytes(N))])
        .await?;
    Ok(slice.try_into()?)
}
