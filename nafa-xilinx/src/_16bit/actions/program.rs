use std::time::{Duration, Instant};

use eyre::Result;
use nafa_io::{Command, units::Bytes};
use smol::future::FutureExt as _;

use crate::_16bit::{Controller, IRCapture, commands};

pub struct ProgramStats {
    pub time_program: Duration,
    pub time_verify: Duration,
    pub success: bool,
}

pub async fn run(mut cont: Controller<'_>, data: &[u8]) -> Result<ProgramStats> {
    let start = Instant::now();
    cont.borrow()
        .run([
            Command::ir(commands::CFG_IN),
            Command::dr_tx_with_notification(data),
            Command::ir(commands::JSTART),
            Command::idle(Bytes(100)),
        ])
        .await?;
    let end_program = Instant::now();

    let status = async {
        loop {
            if let Ok(ir) = cont.borrow().capture_ir().await {
                break IRCapture::from_bits_retain(ir as _).intersects(IRCapture::DONE);
            }
        }
    };
    let success = status
        .or(nafa_io::timeout(Duration::from_millis(100), false))
        .await;
    let end_status = Instant::now();

    Ok(ProgramStats {
        time_program: end_program - start,
        time_verify: end_status - end_program,
        success,
    })
}
