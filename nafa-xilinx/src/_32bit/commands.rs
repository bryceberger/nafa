use nafa_io::units::Bytes;

pub struct Command {
    pub val: u8,
    pub read_len: Bytes<usize>,
}

pub const IDCODE: Command = Command {
    val: 0x09,
    read_len: Bytes(4),
};
pub const FUSE_DNA: Command = Command {
    val: 0x12,
    read_len: Bytes(8),
};
pub const FUSE_KEY: Command = Command {
    val: 0x31,
    read_len: Bytes(32),
};

pub const CFG_IN: u8 = 0x05;
pub const CFG_OUT: u8 = 0x04;
pub const JSTART: u8 = 0x0c;
pub const JPROGRAM: u8 = 0x0b;
pub const XADC_DRP: u8 = 0x37;
