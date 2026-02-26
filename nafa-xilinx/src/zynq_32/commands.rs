pub use self::internal::*;

// from xczu9eg_ffvc900.bsd
#[rustfmt::skip]
mod internal {
    pub const IDCODE:       u32 = 0b_001001_001001; // PS IDCODE, DEVICE_ID reg
    pub const IDCODE_PL:    u32 = 0b_100100_100101; // PRIVATE, PL IDCODE, DEVICE_ID reg
    pub const IDCODE_PSPL:  u32 = 0b_001001_100101; // PRIVATE, PS AND PL IDCODES, DEVICE_ID reg
    pub const BYPASS:       u32 = 0b_111111_111111; // PS BYPASS, BYPASS reg
    pub const EXTEST:       u32 = 0b_100110_100110; // BOUNDARY reg
    pub const SAMPLE:       u32 = 0b_111111_000001; // BOUNDARY reg
    pub const PRELOAD:      u32 = 0b_111111_000001; // BOUNDARY reg, Same as SAMPLE
    pub const USERCODE:     u32 = 0b_100100_001000; // PL USER CODE, DEVICE_ID reg
    pub const HIGHZ_IO:     u32 = 0b_001010_001010; // PRIVATE, BYPASS reg
    pub const JTAG_STATUS:  u32 = 0b_011111_111111; // PRIVATE, STATUS from PS
    pub const JSTATUS:      u32 = 0b_100100_100001; // PRIVATE, STATUS from PL
    pub const EXTEST_PULSE: u32 = 0b_100110_111100; // BOUNDARY reg
    pub const EXTEST_TRAIN: u32 = 0b_100110_111101; // BOUNDARY reg
    pub const ISC_ENABLE:   u32 = 0b_100100_010000; // PRIVATE, ISC_CONFIG
    pub const ISC_PROGRAM:  u32 = 0b_100100_010001; // PRIVATE, ISC_PDATA
    pub const ISC_PROG_SEC: u32 = 0b_100100_010010; // PRIVATE
    pub const ISC_NOOP:     u32 = 0b_100100_010100; // PRIVATE, ISC_DEFAULT
    pub const ISC_DISABLE:  u32 = 0b_100100_010110; // PRIVATE, ISC_CONFIG
    pub const ISC_READ:     u32 = 0b_100100_010101; // PRIVATE, ISC_CONFIG
    pub const XSC_DNA:      u32 = 0b_100100_010111; // PRIVATE, DNA reg
    pub const CFG_IN:       u32 = 0b_100100_000101; // PRIVATE
    pub const CFG_OUT:      u32 = 0b_100100_000100; // PRIVATE
    pub const JPROGRAM:     u32 = 0b_100100_001011; // PRIVATE
    pub const JSTART:       u32 = 0b_100100_001100; // PRIVATE
    pub const JSHUTDOWN:    u32 = 0b_100100_001101; // PRIVATE
    pub const FUSE_CTS:     u32 = 0b_100100_110000; // PRIVATE
    pub const FUSE_KEY:     u32 = 0b_100100_110001; // PRIVATE
    pub const FUSE_DNA:     u32 = 0b_100100_110010; // PRIVATE
    pub const FUSE_CNTL:    u32 = 0b_100100_110100; // PRIVATE
    pub const FUSE_USER_PS: u32 = 0b_001000_111111; // PRIVATE, PS USER CODE, DEVICE_ID reg
    pub const USER1:        u32 = 0b_100100_000010; // PRIVATE, Not available until after configuration
    pub const USER2:        u32 = 0b_100100_000011; // PRIVATE, Not available until after configuration
    pub const USER3:        u32 = 0b_100100_100010; // PRIVATE, Not available until after configuration
    pub const USER4:        u32 = 0b_100100_100011; // PRIVATE, Not available until after configuration
    pub const SYSMON_DRP:   u32 = 0b_100100_110111; // PRIVATE
    pub const JTAG_CTRL:    u32 = 0b_100000_111111; // PRIVATE, JTAG_CTRL reg
    pub const ERROR_STATUS: u32 = 0b_111110_111111; // PRIVATE, PMU ERROR_STATUS reg
    pub const PMU_MDM:      u32 = 0b_000011_111111; // PRIVATE
}
