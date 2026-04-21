use eyre::{OptionExt as _, Result};
use nafa_io::Controller;
use nafa_xilinx::_16bit::actions;

mod program;

#[derive(clap::Subcommand)]
pub enum Command {
    Status,
    Program(program::Args),
}

impl Command {
    pub fn wants_progress(&self) -> bool {
        matches!(self, Command::Program { .. })
    }
}

pub async fn run(
    cont: &mut Controller,
    pb: Option<&indicatif::ProgressBar>,
    command: Command,
) -> Result<Option<Box<dyn FnOnce()>>> {
    let cont = cont
        .typed()
        .ok_or_eyre("cannot call xilinx16 method with non-xilinx16 active device")?;
    match command {
        Command::Status => {
            let status = actions::status::run(cont).await?;
            println!("sswd:         {:01b}", (status >> 15) & 0b1);
            println!("in_pwrdn:     {:01b}", (status >> 14) & 0b1);
            println!("done:         {:01b}", (status >> 13) & 0b1);
            println!("init_b:       {:01b}", (status >> 12) & 0b1);
            println!("mode:         {:03b}", (status >> 9) & 0b111);
            println!("hwshapen:     {:01b}", (status >> 8) & 0b1);
            println!("part_secured: {:01b}", (status >> 7) & 0b1);
            println!("dec_error:    {:01b}", (status >> 6) & 0b1);
            println!("ghigh_b:      {:01b}", (status >> 5) & 0b1);
            println!("gwe:          {:01b}", (status >> 4) & 0b1);
            println!("gts_cfg_b:    {:01b}", (status >> 3) & 0b1);
            println!("dcm_lock:     {:01b}", (status >> 2) & 0b1);
            println!("id_error:     {:01b}", (status >> 1) & 0b1);
            println!("crc_error:    {:01b}", (status >> 0) & 0b1);
            Ok(None)
        }
        Command::Program(args) => program::run(cont, pb, args).await,
    }
}
