use std::time::{Duration, Instant};

use eyre::Result;
use nafa_io::{Command, units::Bytes};
use smol::future::FutureExt as _;

use crate::_32bit::{
    Controller, IRCapture,
    commands::{self, duplicated, shifted},
};

pub struct ProgramStats {
    pub time_shutdown: Duration,
    pub time_program: Duration,
    pub time_verify: Duration,
    pub success: bool,
}

pub async fn run(mut cont: Controller<'_>, data: &[u8]) -> Result<ProgramStats> {
    let num_slr = cont.info().slr;

    let start = Instant::now();
    cont.borrow()
        .run([Command::ir(duplicated(commands::JPROGRAM))])
        .await?;

    while {
        let ir_capture = IRCapture::from_bits_retain(cont.borrow().capture_ir().await? as _);
        !ir_capture.intersects(IRCapture::INIT)
    } {}
    let end_shutdown = Instant::now();

    cont.borrow()
        .run([
            Command::ir(duplicated(commands::JSHUTDOWN)),
            Command::ir(shifted(commands::CFG_IN, num_slr, 0)),
            Command::dr_tx_with_notification(data),
            Command::ir(duplicated(commands::JSTART)),
            Command::idle(Bytes(250)),
        ])
        .await?;
    let end_program = Instant::now();

    let status = async {
        loop {
            // Sometimes, immediately after programming, the FPGA won't respond
            // for a few ms. This surfaces as a "failed to fill buffer", which
            // we ignore in favor of just trying again.
            //
            // Infinite loop is negated by timeout above.
            let Ok(ir) = cont.borrow().capture_ir().await else {
                continue;
            };
            let ir_capture = IRCapture::from_bits_retain(ir as _);
            break ir_capture.intersects(IRCapture::DONE);
        }
    };
    let success = status
        .or(nafa_io::timeout(Duration::from_millis(100), false))
        .await;
    let end_status = Instant::now();

    Ok(ProgramStats {
        time_shutdown: end_shutdown - start,
        time_program: end_program - end_shutdown,
        time_verify: end_status - end_program,
        success,
    })
}
