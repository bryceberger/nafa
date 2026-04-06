use eyre::Result;
use nafa_io::Controller;

mod info;
mod read_idcode;

#[derive(clap::Subcommand)]
pub enum Command {
    Info(info::Args),
    ReadIdCode(read_idcode::Args),
}

pub async fn run(
    cont: &mut Controller,
    command: Command,
) -> Result<Option<Box<dyn FnOnce()>>> {
    let no_action = |()| None;
    match command {
        Command::Info(args) => info::run(cont, args).await.map(no_action),
        Command::ReadIdCode(args) => read_idcode::run(cont, args).await.map(no_action),
    }
}
