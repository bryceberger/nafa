pub use self::Cmd::*;

#[repr(u8)]
#[derive(Clone, Copy)]
#[rustfmt::skip]
pub enum Cmd {
    IDCODE       = 0b00001111,
}
