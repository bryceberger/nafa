use nafa_io::units::Words16;

#[derive(Debug, Clone, Copy)]
pub struct Type1 {
    pub op: OpCode,
    pub addr: Addr,
    pub word_count: Words16<u8>,
}

impl Type1 {
    pub const fn new(op: OpCode, addr: Addr, word_count: Words16<u8>) -> Self {
        Self {
            op,
            addr,
            word_count,
        }
    }
}

#[repr(u8)]
#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OpCode {
    Noop = 0,
    Read = 1,
    Write = 2,
    #[default]
    Unknown,
}

#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Addr {
    Crc = 0,
    FarMajor = 1,
    FarMinor = 2,
    Fdri = 3,
    Fdro = 4,
    Cmd = 5,
    Ctl = 6,
    Mask = 7,
    Stat = 8,
    Lout = 9,
    Cor1 = 10,
    Cor2 = 11,
    Pwrdn = 12,
    Flr = 13,
    Idcode = 14,
    Snowplow = 15,
    HcOpt = 16,
    Csbo = 18,
    General1 = 19,
    General2 = 20,
    Mode = 21,
    PuGwe = 22,
    PuGts = 23,
    Mfwr = 24,
    CclkFreq = 25,
    SeuOpt = 26,
    ExpSign = 27,
    RdbkSign = 28,
}

const fn register_base(type_: u8, opcode: OpCode, addr: Addr, word_count: u8) -> u16 {
    let header = (type_ as u16) << 13;
    let opcode = (((opcode as u8) & 0x3) as u16) << 11;
    let address = (((addr as u8) & 0x3f) as u16) << 5;
    let word_count = (word_count & 0x1f) as u16;
    header | opcode | address | word_count
}

impl Type1 {
    /// ```text
    /// [15:13] header type
    /// [12:11] opcode
    /// [10: 5] address
    /// [ 4: 0] word count
    /// ```
    pub const fn to_raw(self) -> u16 {
        register_base(1, self.op, self.addr, self.word_count.0)
    }

    pub const DUMMY: u16 = 0xffff;
    pub const SYNC: u16 = 0xaa99;
    pub const NOOP: u16 = 0x2000;
}

/// ```text
/// first word:
/// [15:13] header type
/// [12:11] opcode
/// [10: 5] register address
/// [ 4: 0] reserved
///
/// second + third words
/// [31:28] reserved
/// [28:16] word count (high bits)
/// [15: 0] word count (low bits)
/// ```
pub const fn type2(op: OpCode, addr: Addr, word_count: u32) -> [u16; 3] {
    let cmd = register_base(2, op, addr, 0);
    let word_count_high = ((word_count >> 16) & 0x0FFF) as u16;
    let word_count_low = (word_count & 0xFFFF) as u16;
    [cmd, word_count_high, word_count_low]
}
