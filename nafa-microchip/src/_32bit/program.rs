use std::time::Duration;

use eyre::{Result, eyre};
use nafa_io::{self, Backend, Controller, Data, ShortHex, units::Bits};

pub struct ProgramStats {
    pub time_shutdown: Duration,
    pub time_program: Duration,
    pub time_verify: Duration,
    pub success: bool,
}

use crate::_32bit::{S, debug_info, delay, f2_command};

pub async fn run(cont: &mut Controller, data: &[u8]) -> Result<ProgramStats> {
    let (buf, b) = cont.backend();

    // IDCODE Twice
    b.bytes(buf, S.rti_sir, Data::Tx(&[0x0F]), S.sir_sdr)
        .await?;
    b.bytes(buf, None, Data::TxRx(&[0x00; 4]), S.sdr_rti)
        .await?;
    flush_print_clear(buf, b, "IDCODE").await?;
    b.bits(buf, None, 0xFFFFFFFF, Bits(1), S.rti_sdr).await?;
    b.bytes(buf, None, Data::TxRx(&[0x00; 4]), S.sdr_rti)
        .await?;
    flush_print_clear(buf, b, "IDCODE").await?;

    // 0x0D Initialization?
    b.bytes(buf, S.rti_sir, Data::Tx(&[0x0D]), S.sir_rti)
        .await?;
    delay(b, buf, None, S.rti_sdr).await?;
    b.bytes(buf, None, Data::TxRx(&[0x00]), S.sdr_rti).await?;
    flush_print_clear(buf, b, "0x0D Command").await?;

    println!("debug_info: {}", ShortHex(&debug_info::<96>(b, buf).await?));

    // 0xAE Command
    b.bytes(buf, S.rti_sir, Data::Tx(&[0xAE]), S.sir_sdr)
        .await?;
    b.bytes(buf, None, Data::TxRx(&[0x01]), S.sdr_rti).await?;
    flush_print_clear(buf, b, "AE Command").await?;
    delay(b, buf, None, S.rti_sdr).await?;
    b.bytes(buf, None, Data::TxRx(&[0x00]), S.sdr_pdr).await?;
    flush_print_clear(buf, b, "AE Command").await?;

    // EE: if we recieve 0x80 wait flag, resend this command again
    // EE: if we recieve 0x00 (ready), quick send next data
    // quick send data: If we recieve 0x80 then go back to sending
    // EE: if we revcieve 0x0000..02, go into f2 commands for
    // F2: status update - Run until address 0xE. Send EE with previous

    // command EE commands randomly go to 129 bits for some reason

    // Send in bitstream
    enum EEState {
        Program,
        QuickProgram,
    }
    let mut state = EEState::Program;
    let mut data_iter = data.chunks(16).rev().enumerate().peekable();
    let empty: &[u8] = &[0x00; 16];
    while let Some((iteration, chunk)) = data_iter.next() {
        let chunk2 = match data_iter.peek() {
            Some((_, c)) => c,
            None => empty,
        };
        loop {
            match state {
                EEState::Program => ee_command(buf, b, chunk, chunk2).await?,
                EEState::QuickProgram => quick_program(buf, b, chunk).await?,
            }
            b.flush(buf).await?;
            if buf.data()[15] == 0x80 {
                println!("Found 0x80");
                state = EEState::Program;
                let message = format!("{iteration}-program");
                flush_print_clear(buf, b, message.as_str()).await?;
            } else if buf.data() == [0x00; 16] {
                println!("Found zeros");
                state = EEState::QuickProgram;
                let message = format!("{iteration}-quickprogram");
                flush_print_clear(buf, b, message.as_str()).await?;
                break;
            } else if buf.data()[15] == 0x02 {
                println!("Found 0x02");
                for i in 0..16 {
                    f2_command(b, buf, i * 2, S.pdr_sir, S.sdr_pdr).await?;
                }
                state = EEState::Program;
            } else {
                return Err(eyre!(
                    "Loop failed do to unknown buf.data(): {}",
                    ShortHex(buf.data())
                ));
            }
            buf.clear();
        }
    }

    Ok(ProgramStats {
        time_shutdown: Duration::new(1, 0),
        time_program: Duration::new(1, 0),
        time_verify: Duration::new(1, 0),
        success: true,
    })
}

async fn quick_program(
    buf: &mut nafa_io::ScratchBuffer,
    b: &mut dyn Backend,
    input: &[u8],
) -> Result<(), eyre::Error> {
    b.bytes(buf, S.pdr_rti, Data::Tx(&[0x00; 5]), S.rti_sdr)
        .await?;
    b.bytes(buf, None, Data::TxRx(input), S.sdr_pdr).await?;
    Ok(())
}

async fn ee_command(
    buf: &mut nafa_io::ScratchBuffer,
    b: &mut dyn Backend,
    input: &[u8],
    input2: &[u8],
) -> Result<(), eyre::Error> {
    b.bytes(buf, S.pdr_sir, Data::Tx(&[0xEE]), S.sir_sdr)
        .await?;
    b.bytes(buf, None, Data::Tx(input), S.sdr_rti).await?;
    b.bytes(buf, None, Data::Tx(&[0x00; 5]), S.rti_sdr).await?;
    b.bytes(buf, None, Data::TxRx(input2), S.sdr_pdr).await?;
    Ok(())
}

async fn flush_print_clear(
    buf: &mut nafa_io::ScratchBuffer,
    b: &mut dyn Backend,
    label: &str,
) -> Result<(), eyre::Error> {
    b.flush(buf).await?;
    println!("{}: {}", label, ShortHex(buf.data()));
    buf.clear();
    Ok(())
}
