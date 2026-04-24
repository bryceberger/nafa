use eyre::Result;
use nafa_io::Controller;

mod info;
mod program;

#[derive(clap::Subcommand)]
pub enum Command {
    Info(info::Args),
    Program(program::Args),
}

pub async fn run(
    cont: &mut Controller,
    pb: Option<&indicatif::ProgressBar>,
    command: Command,
) -> Result<Option<Box<dyn FnOnce()>>> {
    let no_action = |()| None;
    match command {
        Command::Info(args) => info::run(cont, args).await.map(no_action),
        Command::Program(args) => program::run(cont, pb, args).await,
    }
}
