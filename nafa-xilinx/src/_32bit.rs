use std::{
    task::Poll,
    time::{Duration, Instant},
};

use bitflags::bitflags;
use eyre::Result;
use nafa_io::{
    Command, Controller,
    units::{Bytes, Words32},
};
use smol::future::FutureExt;

use self::{
    commands::{duplicated, master, shifted},
    registers::{Addr, Type1},
};

pub mod bbram;
pub mod commands;
mod crc;
pub mod drp;
pub mod info;
pub mod nky;
pub mod registers;

pub async fn read_device_register(
    cont: &mut Controller,
    num_slr: u8,
    active_slr: u8,
    reg: Type1,
) -> Result<&[u8]> {
    let tiny_bitstream =
        bitstream_to_wire_order([Type1::SYNC, Type1::NOOP, reg.to_raw(), Type1::NOOP, Type1::NOOP]);
    let tiny_bitstream = tiny_bitstream.as_flattened();

    cont.run([
        Command::ir(shifted(commands::CFG_IN, num_slr, active_slr)),
        Command::dr_tx(tiny_bitstream),
        Command::ir(shifted(commands::CFG_OUT, num_slr, active_slr)),
        Command::dr_rx(Bytes::from(reg.word_count.into_())),
    ])
    .await
}

async fn read_device_register_word(
    cont: &mut Controller,
    num_slr: u8,
    active_slr: u8,
    addr: Addr,
) -> Result<u32> {
    let data = read_device_register_sized(cont, num_slr, active_slr, addr).await?;
    let data = from_wire_order(*data);
    Ok(data)
}

async fn read_device_register_sized<const N: usize>(
    cont: &mut Controller,
    num_slr: u8,
    active_slr: u8,
    addr: Addr,
) -> Result<&[u8; N]> {
    const { assert!(N.is_multiple_of(4)) };
    let n = u16::try_from(N / 4).unwrap();
    let slice = read_device_register(
        cont,
        num_slr,
        active_slr,
        Type1::new(registers::OpCode::Read, addr, Words32(n)),
    )
    .await?;
    Ok(slice.try_into()?)
}

async fn read_jtag_register_duplicated<const N: usize>(
    cont: &mut Controller,
    inst: commands::Duplicated,
) -> Result<&[u8; N]> {
    let slice = cont
        .run([Command::ir(duplicated(inst)), Command::dr_rx(Bytes(N))])
        .await?;
    Ok(slice.try_into()?)
}

async fn read_jtag_register_master<const N: usize>(
    cont: &mut Controller,
    num_slr: u8,
    inst: commands::Master,
) -> Result<&[u8; N]> {
    let slice = cont
        .run([Command::ir(master(inst, num_slr)), Command::dr_rx(Bytes(N))])
        .await?;
    Ok(slice.try_into()?)
}

async fn read_jtag_register_shifted<const N: usize>(
    cont: &mut Controller,
    num_slr: u8,
    active_slr: u8,
    inst: commands::Shifted,
) -> Result<&[u8; N]> {
    let slice = cont
        .run([Command::ir(shifted(inst, num_slr, active_slr)), Command::dr_rx(Bytes(N))])
        .await?;
    Ok(slice.try_into()?)
}

pub async fn read_xadc(
    cont: &mut Controller,
    num_slr: u8,
    regs: impl IntoIterator<Item = drp::Command>,
) -> Result<&[u8]> {
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

    cont.run(start.into_iter().chain(drp_commands).chain(after))
        .await
}

bitflags! {
    // note: these bits are reversed from what you might expect reading a BSDL
    // file. This is due to bit 0 being shifted out first, thus ending up on the
    // left-most, thus being the MSB instead of LSB.
    struct IRCapture: u8 {
        const DONE        = 0b000001;
        const INIT        = 0b000010;
        const ISC_ENABLED = 0b000100;
        const ISC_DONE    = 0b001000;
    }
}

pub struct ProgramStats {
    pub time_shutdown: Duration,
    pub time_program: Duration,
    pub time_verify: Duration,
    pub success: bool,
}

pub async fn program(cont: &mut Controller, data: &[u8]) -> Result<ProgramStats> {
    let info = match &cont.info().specific {
        nafa_io::devices::Specific::Xilinx32(info) => info,
        _ => panic!("xilinx programming called with non-xilinx active device"),
    };
    let num_slr = info.slr;

    let start = Instant::now();
    cont.run([Command::ir(duplicated(commands::JPROGRAM))])
        .await?;

    while {
        let ir_capture = IRCapture::from_bits_retain(cont.capture_ir().await? as _);
        !ir_capture.intersects(IRCapture::INIT)
    } {}
    let end_shutdown = Instant::now();

    cont.run([
        Command::ir(duplicated(commands::JSHUTDOWN)),
        Command::ir(shifted(commands::CFG_IN, num_slr, 0)),
        Command::dr_tx_with_notification(data),
        Command::ir(duplicated(commands::JSTART)),
        Command::idle(Bytes(250)),
    ])
    .await?;
    let end_program = Instant::now();

    let stop = end_program + Duration::from_millis(100);
    let timeout = smol::future::poll_fn(move |_| {
        if Instant::now() < stop {
            Poll::Pending
        } else {
            Poll::Ready(false)
        }
    });
    let status = async {
        loop {
            // Sometimes, immediately after programming, the FPGA won't respond
            // for a few ms. This surfaces as a "failed to fill buffer", which
            // we ignore in favor of just trying again.
            //
            // Infinite loop is negated by timeout above.
            let Ok(ir) = cont.capture_ir().await else {
                continue;
            };
            let ir_capture = IRCapture::from_bits_retain(ir as _);
            break ir_capture.intersects(IRCapture::DONE);
        }
    };
    let success = status.or(timeout).await;
    let end_status = Instant::now();

    Ok(ProgramStats {
        time_shutdown: end_shutdown - start,
        time_program: end_program - end_shutdown,
        time_verify: end_status - end_program,
        success,
    })
}

pub async fn readback(cont: &mut Controller, len: Bytes<usize>) -> Result<&[u8]> {
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

pub const fn to_wire_order(x: u32) -> [u8; 4] {
    x.reverse_bits().to_le_bytes()
}

pub const fn from_wire_order(x: [u8; 4]) -> u32 {
    u32::from_le_bytes(x).reverse_bits()
}

pub(crate) fn bitstream_to_wire_order<const N: usize>(x: [u32; N]) -> [[u8; 4]; N] {
    x.map(to_wire_order)
}
