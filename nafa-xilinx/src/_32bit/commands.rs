pub use self::{Duplicated::*, Master::*, Shifted::*};

pub const fn duplicated(val: Duplicated) -> u32 {
    let val = val as u8 as u32;
    val | val << 6 | val << (6 * 2) | val << (6 * 3) | val << (6 * 4) | val << (6 * 5)
}

const DUPLICATED_SKIP_SLR: u32 = 0b100100
    | 0b100100 << 6
    | 0b100100 << (6 * 2)
    | 0b100100 << (6 * 3)
    | 0b100100 << (6 * 4)
    | 0b100100 << (6 * 5);

pub const fn master(val: Master, num_slr: u8) -> u32 {
    let val = val as u8 as u32;
    let shift = (num_slr - 1) * 6;
    DUPLICATED_SKIP_SLR & !(0b111111 << shift) | val << shift
}

pub const fn shifted(val: Shifted, num_slr: u8, active_slr: u8) -> u32 {
    let val = val as u8 as u32;
    let shift = (num_slr - 1 - active_slr) * 6;
    DUPLICATED_SKIP_SLR & !(0b111111 << shift) | val << shift
}

#[repr(u8)]
#[expect(non_camel_case_types)]
#[allow(unused, clippy::upper_case_acronyms)]
#[derive(Clone, Copy)]
#[rustfmt::skip]
pub enum Duplicated {
    IDCODE       = 0b001001,
    BYPASS       = 0b111111,
    EXTEST       = 0b100110,
    SAMPLE       = 0b000001,
    HIGHZ_IO     = 0b001010,
    EXTEST_PULSE = 0b111100,
    EXTEST_TRAIN = 0b111101,
    ISC_ENABLE   = 0b010000,
    ISC_PROGRAM  = 0b010001,
    XSC_PROG_SEC = 0b010010,
    ISC_NOOP     = 0b010100,
    ISC_READ     = 0b010101,
    ISC_DISABLE  = 0b010110,
    JPROGRAM     = 0b001011,
    JSTART       = 0b001100,
    JSHUTDOWN    = 0b001101,
    JSTATUS      = 0b100001,
}

#[repr(u8)]
#[expect(non_camel_case_types)]
#[allow(unused, clippy::upper_case_acronyms)]
#[derive(Clone, Copy)]
#[rustfmt::skip]
pub enum Master {
    USERCODE      = 0b001000,
    XSC_DNA       = 0b010111,
    FUSE_USER     = 0b110011,
    FUSE_USER_128 = 0b011001,
    USER1         = 0b000010,
    USER2         = 0b000011,
    USER3         = 0b100010,
    USER4         = 0b100011,
    SYSMON_DRP    = 0b110111,
}

#[repr(u8)]
#[expect(non_camel_case_types)]
#[allow(unused, clippy::upper_case_acronyms)]
#[derive(Clone, Copy)]
#[rustfmt::skip]
pub enum Shifted {
    XSC_PROGRAM = 0b010001,
    CFG_OUT     = 0b000100,
    CFG_IN      = 0b000101,
    FUSE_CTS    = 0b110000,
    FUSE_KEY    = 0b110001,
    FUSE_DNA    = 0b110010,
    FUSE_CNTL   = 0b110100,
    FUSE_RSA    = 0b011000,
    FUSE_SEC    = 0b111011,
}
