pub use self::internal::*;

// from XC7VX690T.bsdl
#[rustfmt::skip]
mod internal {
    pub const IDCODE:          u8 = 0b001001; // DEVICE_ID
    pub const BYPASS:          u8 = 0b111111; // BYPASS
    pub const EXTEST:          u8 = 0b100110; // BOUNDARY
    pub const SAMPLE:          u8 = 0b000001; // BOUNDARY
    pub const PRELOAD:         u8 = 0b000001; // Same as SAMPLE
    pub const USERCODE:        u8 = 0b001000; // DEVICE_ID
    pub const HIGHZ:           u8 = 0b001010; // BYPASS
    pub const EXTEST_PULSE:    u8 = 0b111100; // BOUNDARY
    pub const EXTEST_TRAIN:    u8 = 0b111101; // BOUNDARY
    pub const ISC_ENABLE:      u8 = 0b010000; // ISC_CONFIG
    pub const ISC_PROGRAM:     u8 = 0b010001; // ISC_PDATA
    pub const ISC_NOOP:        u8 = 0b010100; // ISC_DEFAULT
    pub const XSC_READ_RSVD:   u8 = 0b010101; // PRIVATE
    pub const ISC_DISABLE:     u8 = 0b010110; // ISC_CONFIG
    pub const XSC_PROGRAM_KEY: u8 = 0b010010; // XSC_KEY_DATA
    pub const XSC_DNA:         u8 = 0b010111; // DNA
    pub const CFG_OUT:         u8 = 0b000100; // Not available during configuration with another mode.
    pub const CFG_IN:          u8 = 0b000101; // Not available during configuration with another mode.
    pub const JPROGRAM:        u8 = 0b001011; // Not available during configuration with another mode.
    pub const JSTART:          u8 = 0b001100; // Not available during configuration with another mode.
    pub const JSHUTDOWN:       u8 = 0b001101; // Not available during configuration with another mode.
    pub const FUSE_CTS:        u8 = 0b110000; // PRIVATE
    pub const FUSE_KEY:        u8 = 0b110001; // PRIVATE
    pub const FUSE_DNA:        u8 = 0b110010; // PRIVATE
    pub const FUSE_USER:       u8 = 0b110011; // PRIVATE
    pub const FUSE_USER_128:   u8 = 0b011001; // PRIVATE
    pub const FUSE_CNTL:       u8 = 0b110100; // PRIVATE
    pub const USER1:           u8 = 0b000010; // Not available until after configuration
    pub const USER2:           u8 = 0b000011; // Not available until after configuration
    pub const USER3:           u8 = 0b100010; // Not available until after configuration
    pub const USER4:           u8 = 0b100011; // Not available until after configuration
    pub const XADC_DRP:        u8 = 0b110111; // PRIVATE
    pub const INTEST_RSVD:     u8 = 0b000111; // PRIVATE
}
