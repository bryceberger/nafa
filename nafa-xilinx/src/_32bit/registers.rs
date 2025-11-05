#[derive(Debug, Clone, Copy)]
pub struct Type1 {
    pub op: OpCode,
    pub addr: Addr,
    pub word_count: u16,
}

impl Type1 {
    pub const fn new(op: OpCode, addr: Addr, word_count: u16) -> Self {
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

#[repr(u16)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Addr {
    Crc = 0,
    Far = 1,
    Fdri = 2,
    Fdro = 3,
    Cmd = 4,
    Ctl0 = 5,
    Mask = 6,
    Stat = 7,
    Lout = 8,
    Cor0 = 9,
    Mfwr = 10,
    Cbc = 11,
    Idcode = 12,
    Axss = 13,
    Cor1 = 14,
    Wbstar = 16,
    Timer = 17,
    RbcrcSw = 19,
    Bootsts = 22,
    Ctl1 = 24,
    Rdri = 26,
    Ssit = 30,
    Bspi = 31,
}

impl Type1 {
    /// ```text
    /// [31:29] header type
    /// [28:27] opcode
    /// [26:13] address
    /// [12:11] reserved
    /// [10: 0] word count
    /// ```
    pub const fn to_raw(self) -> u32 {
        let header = 1 << 29;
        let opcode = (((self.op as u8) & 0x3) as u32) << 27;
        let address = (((self.addr as u16) & 0x3fff) as u32) << 13;
        let word_count = (self.word_count & 0x3ff) as u32;
        header | opcode | address | word_count
    }

    pub const DUMMY: u32 = 0xffffffff;
    pub const SYNC: u32 = 0xaa995566;
    pub const NOOP: u32 = 0x20000000;
}

/// ```text
/// [31:29] header type
/// [28:27] opcode
/// [26: 0] word count
/// ```
pub const fn type2(op: OpCode, word_count: u32) -> u32 {
    let header = 2 << 29;
    let opcode = ((op as u8 & 0x3) as u32) << 27;
    let word_count = word_count & 0x03ff_ffff;
    header | opcode | word_count
}
