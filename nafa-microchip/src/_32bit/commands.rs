pub use self::Cmd::*;

#[repr(u8)]
#[derive(Clone, Copy)]
#[rustfmt::skip]
pub enum Cmd {
    IDCODE       = 0b00001111,
    SILSIG       = 0b00001110,
    DSN          = 0b11110000,
}
