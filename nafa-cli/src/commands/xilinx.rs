use eyre::Result;
use nafa_io::Controller;

mod info;
mod program;
mod program_bbram;
mod readback;
mod xadc;

#[derive(clap::Subcommand)]
pub enum Command {
    Info(info::Args),
    Xadc(xadc::Args),
    Readback(readback::Args),
    Program(program::Args),
    ProgramBbramKey(program_bbram::Args),
}

impl Command {
    pub fn wants_progress(&self) -> bool {
        matches!(self, Command::Readback(_) | Command::Program(_))
    }
}

pub async fn run(
    cont: &mut Controller,
    pb: Option<&indicatif::ProgressBar>,
    command: Command,
) -> Result<Option<Box<dyn FnOnce()>>> {
    let no_action = |()| None;
    match command {
        Command::Info(args) => info::run(cont, args).await.map(no_action),
        Command::Xadc(args) => xadc::run(cont, args).await.map(no_action),
        Command::Readback(args) => readback::run(cont, pb, args).await,
        Command::Program(args) => program::run(cont, pb, args).await,
        Command::ProgramBbramKey(args) => program_bbram::run(cont, args).await.map(no_action),
    }
}
