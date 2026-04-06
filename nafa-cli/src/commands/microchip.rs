use eyre::Result;
use nafa_io::Controller;

mod info;

#[derive(clap::Subcommand)]
pub enum Command {
    Info(info::Args),
}

pub async fn run(cont: &mut Controller, command: Command) -> Result<Option<Box<dyn FnOnce()>>> {
    let no_action = |()| None;
    match command {
        Command::Info(args) => info::run(cont, args).await.map(no_action),
    }
}
