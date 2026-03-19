use eyre::Result;
use nafa_io::{Backend, Controller};

pub async fn program_key(
    cont: &mut Controller<impl Backend>,
    keys: &[[u8; 32]],
    dpa: Option<Dpa>,
) -> Result<()> {
    todo!()
}

#[derive(Clone, Copy)]
pub enum DpaMode {
    Invalid,
    All,
}

#[derive(Clone, Copy)]
pub struct Dpa {
    pub mode: DpaMode,
    pub count: u16,
}
