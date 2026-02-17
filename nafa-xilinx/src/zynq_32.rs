//! Utilities to read from Zynq devices.
//!
//! These are _almost_ the same as the rest of the devices. However, they have
//! an IRLEN of 12 instead of 6 --- there's a processor and FPGA stuck together
//! acting as a single device.
//!
//! You'd _think_ that this would mean you send a noop to the processor and the
//! actual command to the FPGA. However, for some reason, that gives garbled
//! data. Instead, we duplicate the command and send it to both devices.
//! Currently unknown _why_ this works, or if it works for all commands.
//!
//! The following strategies were tried:
//!
//! | strategy    | idcode | efuse |
//! |-------------|--------|-------|
//! | noop low    |   ✕    |   ✕   |
//! | noop high   |   ✕    |   ✓   |
//! | bypass low  |   ✓    |   ✕   |
//! | bypass high |   ✕    |   ✓   |
//! | replicated  |   ✓    |   ✓   |

use eyre::Result;
use nafa_io::{
    Backend, Command, Controller,
    units::{Bytes, Words32},
};

use crate::_32bit::{
    bitstream_to_wire_order, commands,
    registers::{Addr, OpCode, Type1},
};

pub mod info;

fn duplicate(cmd: u8) -> u32 {
    let cmd = u32::from(cmd & 0b111111);
    cmd << 6 | cmd
}

pub async fn read_device_register(
    cont: &mut Controller<impl Backend>,
    reg: Type1,
) -> Result<&[u8]> {
    let tiny_bitstream =
        bitstream_to_wire_order([Type1::SYNC, Type1::NOOP, reg.to_raw(), Type1::NOOP, Type1::NOOP]);
    let tiny_bitstream = tiny_bitstream.as_flattened();

    cont.run([
        Command::ir(commands::CFG_IN as _),
        Command::dr_tx(tiny_bitstream),
        Command::ir(commands::CFG_OUT as _),
        Command::dr_rx(Bytes::from(reg.word_count.into_())),
    ])
    .await
}

async fn read_device_register_word(cont: &mut Controller<impl Backend>, addr: Addr) -> Result<u32> {
    let data = read_device_register_sized(cont, addr).await?;
    let data = data.map(|x| x.reverse_bits());
    Ok(u32::from_be_bytes(data))
}

async fn read_device_register_sized<const N: usize>(
    cont: &mut Controller<impl Backend>,
    addr: Addr,
) -> Result<&[u8; N]> {
    const { assert!(N.is_multiple_of(4)) };
    let n = u16::try_from(N / 4).unwrap();
    read_device_register(cont, Type1::new(OpCode::Read, addr, Words32(n)))
        .await
        .map(|x| {
            x.try_into()
                .expect("dr_rx() should always return exact len")
        })
}

async fn read_jtag_register<B: Backend>(
    cont: &mut Controller<B>,
    inst: u8,
    len: Bytes<usize>,
) -> Result<&[u8]> {
    cont.run([Command::ir(duplicate(inst)), Command::dr_rx(len)])
        .await
}

async fn read_jtag_register_sized<const N: usize, B: Backend>(
    cont: &mut Controller<B>,
    inst: u8,
) -> Result<&[u8; N]> {
    read_jtag_register(cont, inst, Bytes(N)).await.map(|x| {
        x.try_into()
            .expect("dr_rx() should always return exact len")
    })
}
