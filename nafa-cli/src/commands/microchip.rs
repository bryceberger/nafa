use eyre::Result;
use nafa_io::{
    Backend, Controller, Data, ScratchBuffer, ShortHex,
    jtag::{PATHS, State},
    units::Bits,
};

mod info;

#[derive(clap::Subcommand)]
pub enum Command {
    Info(info::Args),
    Hardcoded,
}

pub async fn run(cont: &mut Controller, command: Command) -> Result<Option<Box<dyn FnOnce()>>> {
    let no_action = |()| None;
    match command {
        Command::Info(args) => info::run(cont, args).await.map(no_action),
        Command::Hardcoded => {
            let (buf, backend) = cont.backend();
            weird_microchip_rti(backend, buf).await?;
            Ok(None)
        }
    }
}

pub async fn weird_microchip_rti(b: &mut dyn Backend, buf: &mut ScratchBuffer) -> Result<()> {
    let pdr_sir = Some(PATHS[State::PauseDR][State::ShiftIR]);
    let rti_sdr = Some(PATHS[State::RunTestIdle][State::ShiftDR]);
    let rti_sir = Some(PATHS[State::RunTestIdle][State::ShiftIR]);
    let sdr_pdr = Some(PATHS[State::ShiftDR][State::PauseDR]);
    let sdr_pir = Some(PATHS[State::ShiftDR][State::PauseIR]);
    let sdr_rti = Some(PATHS[State::ShiftDR][State::RunTestIdle]);
    let sir_pir = Some(PATHS[State::ShiftIR][State::PauseIR]);
    let sir_sdr = Some(PATHS[State::ShiftIR][State::ShiftDR]);
    let pir_sdr = Some(PATHS[State::PauseIR][State::ShiftDR]);

    // UDV
    loop {
        b.bytes(buf, rti_sir, Data::Tx(&[0xCD]), sir_pir).await?;
        delay(b, buf, None).await?;
        b.bytes(buf, pir_sdr, Data::TxRx(&[0x00; 4]), sdr_pdr)
            .await?;
        b.flush(buf).await?;
        if buf.data()[3] != 0x80 {
            println!("UDV: {}", ShortHex(buf.data()));
            buf.clear();
            break;
        }
        buf.clear();
    }

    // SILSIG
    simple_command(b, buf, 0x0E, pdr_sir, sdr_pdr).await?;
    flush_print(b, buf, "SILSIG").await?;

    // IDCODE 0F
    simple_command(b, buf, 0x0F, pdr_sir, sdr_rti).await?;
    flush_print(b, buf, "IDCODE").await?;

    // READ_DESIGN_INFO
    // Weird A6 command
    complex_command(b, buf, 0xA6, rti_sir, sdr_pdr, [0x00].to_vec()).await?;
    // Weird A6 command pt 2
    b.bytes(buf, pdr_sir, Data::Tx(&[0xA6]), sir_pir).await?;
    b.bytes(buf, pir_sdr, Data::TxRx(&[0x00]), sdr_rti).await?;
    // F2
    for i in 0..3 {
        f2_command(b, buf, i * 2, rti_sir, sdr_rti).await?;
    }
    flush_print(b, buf, "READ_DESIGN_INFO").await?;

    // Digests
    complex_command(b, buf, 0xA3, rti_sir, sdr_pdr, [0x00].to_vec()).await?;
    b.flush(buf).await?;
    buf.clear();
    loop {
        b.bytes(buf, pdr_sir, Data::Tx(&[0xA3]), sir_pir).await?;
        delay(b, buf, pir_sdr).await?;
        b.bytes(buf, None, Data::TxRx(&[0x00]), sdr_pdr).await?;
        b.flush(buf).await?;
        if *buf.data() == [0x00] {
            buf.clear();
            break;
        }
        buf.clear();
    }
    let digests = [
        "CHECK_FABRIC_digest",
        "CC_digest",
        "SNVM_digest",
        "UL_digest",
        "UKDIGEST0",
        "UKDIGEST1",
        "UKDIGEST2",
        "UKDIGEST3",
        "UKDIGEST4",
        "UKDIGEST5",
        "UKDIGEST6",
        "UPERM_digest",
        "SYS_digest",
    ];
    for (i, d) in digests.iter().enumerate() {
        let address: u8 = i as u8 * 4;
        let address2: u8 = i as u8 * 4 + 2;
        f2_command(b, buf, address, pdr_sir, sdr_pdr).await?;
        f2_command(b, buf, address2, pdr_sir, sdr_pdr).await?;
        flush_print(b, buf, d).await?;
    }

    // Device integrity bits & device serial number
    let e1_address: [u8; 16] = [
        0xB4, 0x70, 0xD8, 0x05, 0x01, 0x4F, 0x1C, 0x77, 0xDE, 0x47, 0x9E, 0xCE, 0x6A, 0x31, 0x72,
        0x5C,
    ];
    b.bytes(buf, pdr_sir, Data::Tx(&[0xE1]), sir_pir).await?;
    delay(b, buf, pir_sdr).await?;
    b.bytes(buf, None, Data::Tx(&e1_address), sdr_rti).await?;
    delay(b, buf, rti_sdr).await?;
    b.bytes(buf, None, Data::TxRx(&e1_address), sdr_pdr).await?;
    b.flush(buf).await?;
    buf.clear();
    loop {
        b.bytes(buf, pdr_sir, Data::Tx(&[0xE1]), sir_pir).await?;
        b.bytes(buf, pir_sdr, Data::TxRx(&e1_address), sdr_pdr)
            .await?;
        b.flush(buf).await?;
        if *buf.data() == [0x00; 16] {
            buf.clear();
            break;
        }
        buf.clear();
    }
    f2_command(b, buf, 0x00, pdr_sir, sdr_pdr).await?;
    f2_command(b, buf, 0x02, pdr_sir, sdr_pdr).await?;
    flush_print(b, buf, "devive_integrity_bits").await?;
    f2_command(b, buf, 0x04, pdr_sir, sdr_pdr).await?;
    flush_print(b, buf, "Device_Serial_Number").await?;

    // DEBUG_INFO
    complex_command(b, buf, 0xE7, pdr_sir, sdr_pdr, vec![0x00; 16]).await?;
    loop {
        b.bytes(buf, pdr_sir, Data::Tx(&[0xE7]), sir_pir).await?;
        delay(b, buf, pir_sdr).await?;
        b.bytes(buf, None, Data::TxRx(&[0x00; 16]), sdr_pdr).await?;
        b.flush(buf).await?;
        if *buf.data() == [0x00; 16] {
            buf.clear();
            break;
        }
        buf.clear();
    }
    for i in 0..6 {
        f2_command(b, buf, i * 2, pdr_sir, sdr_pdr).await?;
    }
    // TODO: Cut off extra bytes over 672 bits
    flush_print(b, buf, "DEBUG_INFO").await?;

    // QUERY_SECURITY
    complex_command(b, buf, 0xB8, pdr_sir, sdr_pdr, vec![0x00; 2]).await?;
    buf.clear();
    loop {
        b.bytes(buf, pdr_sir, Data::Tx(&[0xB8]), sir_sdr).await?;
        b.bytes(buf, None, Data::Tx(&[0x00; 2]), sdr_pir).await?;
        b.bytes(buf, pir_sdr, Data::TxRx(&[0x00; 2]), sdr_pdr)
            .await?;
        b.flush(buf).await?;
        if buf.data()[1] != 0x80 {
            buf.clear();
            break;
        }
        buf.clear();
    }
    f2_command(b, buf, 0x00, pdr_sir, sdr_pdr).await?;
    flush_print(b, buf, "QUERY_SECURITY").await?;

    Ok(())
}

async fn delay(
    b: &mut dyn Backend,
    buf: &mut ScratchBuffer,
    after: Option<nafa_io::jtag::Path>,
) -> Result<(), eyre::Error> {
    b.bits(buf, None, 0xFFFFFFFF, Bits(3), None).await?;
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
    let sir_sdr = Some(PATHS[State::ShiftIR][State::ShiftDR]);
    b.bytes(buf, before, Data::Tx(&[command]), sir_sdr).await?;
    b.bytes(buf, None, Data::TxRx(&[0x00; 4]), after).await?;
    Ok(())
}

async fn flush_print(
    b: &mut dyn Backend,
    buf: &mut ScratchBuffer,
    name: &str,
) -> Result<(), eyre::Error> {
    b.flush(buf).await?;
    println!("{}: {}", name, ShortHex(buf.data()));
    buf.clear();
    Ok(())
}

async fn f2_command(
    b: &mut dyn Backend,
    buf: &mut ScratchBuffer,
    address: u8,
    before: Option<nafa_io::jtag::Path>,
    after: Option<nafa_io::jtag::Path>,
) -> Result<(), eyre::Error> {
    let rti_sdr = Some(PATHS[State::RunTestIdle][State::ShiftDR]);
    let addresses = [
        address, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00,
    ];
    let sdr_rti = Some(PATHS[State::ShiftDR][State::RunTestIdle]);
    let sir_sdr = Some(PATHS[State::ShiftIR][State::ShiftDR]);
    //send the f2 command & address
    b.bytes(buf, before, Data::Tx(&[0xF2]), sir_sdr).await?;
    b.bytes(buf, None, Data::Tx(&addresses), None).await?;
    // Microchips weird activate thing
    b.bits(buf, None, 0xFFFFFFFF, Bits(1), sdr_rti).await?;
    delay(b, buf, rti_sdr).await?;
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
    let rti_sdr = Some(PATHS[State::RunTestIdle][State::ShiftDR]);
    let sdr_rti = Some(PATHS[State::ShiftDR][State::RunTestIdle]);
    let sir_sdr = Some(PATHS[State::ShiftIR][State::ShiftDR]);
    b.bytes(buf, before, Data::Tx(&[inst]), sir_sdr).await?;
    b.bytes(buf, None, Data::Tx(&address), sdr_rti).await?;
    delay(b, buf, rti_sdr).await?;
    b.bytes(buf, None, Data::TxRx(&address), after).await?;
    Ok(())
}
