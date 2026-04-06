use std::sync::LazyLock;

use eyre::Result;
use nafa_io::{
    Backend, Data, ScratchBuffer,
    jtag::{PATHS, State},
    units::Bits,
};
use zerocopy::FromBytes;

use crate::_32bit::info::Digests;

pub mod info;

pub struct Transition {
    pdr_sir: Option<nafa_io::jtag::Path>,
    rti_sdr: Option<nafa_io::jtag::Path>,
    rti_sir: Option<nafa_io::jtag::Path>,
    sdr_pdr: Option<nafa_io::jtag::Path>,
    sdr_pir: Option<nafa_io::jtag::Path>,
    sdr_rti: Option<nafa_io::jtag::Path>,
    sir_pir: Option<nafa_io::jtag::Path>,
    sir_sdr: Option<nafa_io::jtag::Path>,
    pir_sdr: Option<nafa_io::jtag::Path>,
}
static S: LazyLock<Transition> = LazyLock::new(|| Transition {
    pdr_sir: Some(PATHS[State::PauseDR][State::ShiftIR]),
    rti_sdr: Some(PATHS[State::RunTestIdle][State::ShiftDR]),
    rti_sir: Some(PATHS[State::RunTestIdle][State::ShiftIR]),
    sdr_pdr: Some(PATHS[State::ShiftDR][State::PauseDR]),
    sdr_pir: Some(PATHS[State::ShiftDR][State::PauseIR]),
    sdr_rti: Some(PATHS[State::ShiftDR][State::RunTestIdle]),
    sir_pir: Some(PATHS[State::ShiftIR][State::PauseIR]),
    sir_sdr: Some(PATHS[State::ShiftIR][State::ShiftDR]),
    pir_sdr: Some(PATHS[State::PauseIR][State::ShiftDR]),
});

async fn idcode<const N: usize>(b: &mut dyn Backend, buf: &mut ScratchBuffer) -> Result<[u8; N]> {
    buf.clear();
    simple_command(b, buf, 0x0F, S.rti_sir, S.sdr_rti).await?;
    b.flush(buf).await?;
    Ok(buf.data().try_into()?)
}

async fn udv<const N: usize>(b: &mut dyn Backend, buf: &mut ScratchBuffer) -> Result<[u8; N]> {
    buf.clear();
    b.bytes(buf, S.rti_sir, Data::Tx(&[0xCD]), S.sir_sdr)
        .await?;
    b.bytes(buf, None, Data::TxRx(&[0x00; 4]), S.sdr_rti)
        .await?;
    delay(b, buf, None, S.rti_sdr).await?;
    b.bytes(buf, None, Data::TxRx(&[0x00; 4]), S.sdr_pdr)
        .await?;
    b.flush(buf).await?;
    buf.clear();
    let data = loop {
        b.bytes(buf, S.pdr_sir, Data::Tx(&[0xCD]), S.sir_pir)
            .await?;
        delay(b, buf, None, None).await?;
        b.bytes(buf, S.pir_sdr, Data::TxRx(&[0x00; 4]), S.sdr_pdr)
            .await?;
        b.flush(buf).await?;
        let data = buf.data();
        if data[3] != 0x80 {
            break data.try_into()?;
        }
        buf.clear();
    };
    pdr_to_rti(b, buf).await?;
    Ok(data)
}

async fn silsig<const N: usize>(b: &mut dyn Backend, buf: &mut ScratchBuffer) -> Result<[u8; N]> {
    buf.clear();
    simple_command(b, buf, 0x0E, S.rti_sir, S.sdr_rti).await?;
    b.flush(buf).await?;
    Ok(buf.data().try_into()?)
}

async fn read_design_info<const N: usize>(
    b: &mut dyn Backend,
    buf: &mut ScratchBuffer,
) -> Result<[u8; N]> {
    buf.clear();
    b.bytes(buf, S.rti_sir, Data::Tx(&[0xA6]), S.sir_sdr)
        .await?;
    b.bytes(buf, None, Data::TxRx(&[0x00]), S.sdr_rti).await?;
    delay(b, buf, None, S.rti_sdr).await?;
    b.bytes(buf, None, Data::TxRx(&[0x00]), S.sdr_pdr).await?;
    b.bytes(buf, S.pdr_sir, Data::Tx(&[0xA6]), S.sir_pir)
        .await?;
    delay(b, buf, None, None).await?;
    b.bytes(buf, S.pir_sdr, Data::TxRx(&[0x00]), S.sdr_pdr)
        .await?;
    b.flush(buf).await?;
    buf.clear();
    for i in 0..3 {
        f2_command(b, buf, i * 2, S.pdr_sir, S.sdr_pdr).await?;
    }
    b.flush(buf).await?;
    let data: [u8; N] = buf.data().try_into()?;
    pdr_to_rti(b, buf).await?;
    Ok(data)
}

async fn digests(b: &mut dyn Backend, buf: &mut ScratchBuffer) -> Result<info::Digests> {
    buf.clear();
    complex_command(b, buf, 0xA3, S.rti_sir, S.sdr_pdr, [0x00].to_vec()).await?;
    b.flush(buf).await?;
    buf.clear();
    loop {
        b.bytes(buf, S.pdr_sir, Data::Tx(&[0xA3]), S.sir_pir)
            .await?;
        delay(b, buf, None, S.pir_sdr).await?;
        b.bytes(buf, None, Data::TxRx(&[0x00]), S.sdr_pdr).await?;
        b.flush(buf).await?;
        if *buf.data() == [0x00] {
            buf.clear();
            break;
        }
        buf.clear();
    }
    for i in 0..13 {
        let address: u8 = i as u8 * 4;
        let address2: u8 = i as u8 * 4 + 2;
        f2_command(b, buf, address, S.pdr_sir, S.sdr_pdr).await?;
        f2_command(b, buf, address2, S.pdr_sir, S.sdr_pdr).await?;
    }
    b.flush(buf).await?;
    let data = Digests::read_from_bytes(buf.data()).unwrap();
    pdr_to_rti(b, buf).await?;
    Ok(data)
}

pub struct E1Command {
    pub device_integrity_bits: [u8; 32],
    pub dsn: [u8; 16],
}

async fn device_integrity_and_dsn<const N: usize, const T: usize>(
    b: &mut dyn Backend,
    buf: &mut ScratchBuffer,
) -> Result<E1Command> {
    buf.clear();
    let e1_address: [u8; 16] = [
        0xB4, 0x70, 0xD8, 0x05, 0x01, 0x4F, 0x1C, 0x77, 0xDE, 0x47, 0x9E, 0xCE, 0x6A, 0x31, 0x72,
        0x5C,
    ];
    b.bytes(buf, S.rti_sir, Data::Tx(&[0xE1]), S.sir_pir)
        .await?;
    delay(b, buf, None, S.pir_sdr).await?;
    b.bytes(buf, None, Data::Tx(&e1_address), S.sdr_rti).await?;
    delay(b, buf, None, S.rti_sdr).await?;
    b.bytes(buf, None, Data::TxRx(&e1_address), S.sdr_pdr)
        .await?;
    b.flush(buf).await?;
    buf.clear();
    loop {
        b.bytes(buf, S.pdr_sir, Data::Tx(&[0xE1]), S.sir_pir)
            .await?;
        b.bytes(buf, S.pir_sdr, Data::TxRx(&e1_address), S.sdr_pdr)
            .await?;
        b.flush(buf).await?;
        if *buf.data() == [0x00; 16] {
            buf.clear();
            break;
        }
        buf.clear();
    }
    f2_command(b, buf, 0x00, S.pdr_sir, S.sdr_pdr).await?;
    f2_command(b, buf, 0x02, S.pdr_sir, S.sdr_pdr).await?;
    b.flush(buf).await?;
    let device_integrity_bits = buf.data().try_into()?;
    buf.clear();
    f2_command(b, buf, 0x04, S.pdr_sir, S.sdr_pdr).await?;
    b.flush(buf).await?;
    let dsn = buf.data().try_into()?;
    pdr_to_rti(b, buf).await?;
    Ok(E1Command {
        device_integrity_bits,
        dsn,
    })
}

async fn debug_info<const N: usize>(
    b: &mut dyn Backend,
    buf: &mut ScratchBuffer,
) -> Result<[u8; N]> {
    buf.clear();
    complex_command(b, buf, 0xE7, S.rti_sir, S.sdr_pdr, vec![0x00; 16]).await?;
    loop {
        b.bytes(buf, S.pdr_sir, Data::Tx(&[0xE7]), S.sir_pir)
            .await?;
        delay(b, buf, None, S.pir_sdr).await?;
        b.bytes(buf, None, Data::TxRx(&[0x00; 16]), S.sdr_pdr)
            .await?;
        b.flush(buf).await?;
        if *buf.data() == [0x00; 16] {
            buf.clear();
            break;
        }
        buf.clear();
    }
    for i in 0..6 {
        f2_command(b, buf, i * 2, S.pdr_sir, S.sdr_pdr).await?;
    }
    b.flush(buf).await?;
    // this will give 96 hex charecters
    let data = buf.data().try_into()?;
    pdr_to_rti(b, buf).await?;
    Ok(data)
}

pub async fn query_security<const N: usize>(
    b: &mut dyn Backend,
    buf: &mut ScratchBuffer,
) -> Result<[u8; N]> {
    buf.clear();
    complex_command(b, buf, 0xB8, S.rti_sir, S.sdr_pdr, vec![0x00; 2]).await?;
    buf.clear();
    loop {
        b.bytes(buf, S.pdr_sir, Data::Tx(&[0xB8]), S.sir_sdr)
            .await?;
        b.bytes(buf, None, Data::Tx(&[0x00; 2]), S.sdr_pir).await?;
        b.bytes(buf, S.pir_sdr, Data::TxRx(&[0x00; 2]), S.sdr_pdr)
            .await?;
        b.flush(buf).await?;
        if buf.data()[1] != 0x80 {
            buf.clear();
            break;
        }
        buf.clear();
    }
    f2_command(b, buf, 0x00, S.pdr_sir, S.sdr_pdr).await?;
    b.flush(buf).await?;
    let data = buf.data().try_into()?;
    pdr_to_rti(b, buf).await?;
    Ok(data)
}

async fn pdr_to_rti(b: &mut dyn Backend, buf: &mut ScratchBuffer) -> Result<(), eyre::Error> {
    simple_command(b, buf, 0x0F, S.pdr_sir, S.sdr_rti).await?;
    b.flush(buf).await?;
    buf.clear();
    Ok(())
}

async fn delay(
    b: &mut dyn Backend,
    buf: &mut ScratchBuffer,
    before: Option<nafa_io::jtag::Path>,
    after: Option<nafa_io::jtag::Path>,
) -> Result<(), eyre::Error> {
    b.bits(buf, before, 0xFFFFFFFF, Bits(3), None).await?;
    b.bytes(buf, None, Data::Tx(&[0x00; 5]), after).await?;
    Ok(())
}

async fn simple_command(
    b: &mut dyn Backend,
    buf: &mut ScratchBuffer,
    command: u8,
    before: Option<nafa_io::jtag::Path>,
    after: Option<nafa_io::jtag::Path>,
) -> Result<(), eyre::Error> {
    b.bytes(buf, before, Data::Tx(&[command]), S.sir_sdr)
        .await?;
    b.bytes(buf, None, Data::TxRx(&[0x00; 4]), after).await?;
    Ok(())
}

async fn f2_command(
    b: &mut dyn Backend,
    buf: &mut ScratchBuffer,
    address: u8,
    before: Option<nafa_io::jtag::Path>,
    after: Option<nafa_io::jtag::Path>,
) -> Result<(), eyre::Error> {
    let addresses = [
        address, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ];
    //send the f2 command & address
    b.bytes(buf, before, Data::Tx(&[0xF2]), S.sir_sdr).await?;
    b.bytes(buf, None, Data::Tx(&addresses), None).await?;
    // Microchips weird activate thing
    b.bits(buf, None, 0xFFFFFFFF, Bits(1), S.sdr_rti).await?;
    delay(b, buf, None, S.rti_sdr).await?;
    // Recieve Data
    b.bytes(buf, None, Data::TxRx(&[0x00; 16]), None).await?;
    b.bits(buf, None, 0xFFFFFFFF, Bits(1), after).await?;
    Ok(())
}

async fn complex_command(
    b: &mut dyn Backend,
    buf: &mut ScratchBuffer,
    inst: u8,
    before: Option<nafa_io::jtag::Path>,
    after: Option<nafa_io::jtag::Path>,
    address: Vec<u8>,
) -> Result<(), eyre::Error> {
    b.bytes(buf, before, Data::Tx(&[inst]), S.sir_sdr).await?;
    b.bytes(buf, None, Data::Tx(&address), S.sdr_rti).await?;
    delay(b, buf, None, S.rti_sdr).await?;
    b.bytes(buf, None, Data::TxRx(&address), after).await?;
    Ok(())
}
