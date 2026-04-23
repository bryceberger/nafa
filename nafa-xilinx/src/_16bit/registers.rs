#![allow(unused)]

use nafa_io::units::Words16;

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum OpCode {
    Noop = 0b00,
    Read = 0b01,
    Write = 0b10,
}

#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Addr {
    /// Cyclic Redundancy Check.
    Crc = 0x00,
    /// Frame Address Register Block and Major.
    FarMaj = 0x01,
    /// Frame Address Register Minor.
    FarMin = 0x02,
    /// Frame Data Input.
    Fdri = 0x03,
    /// Frame Data Output.
    Fdro = 0x04,
    /// Command.
    Cmd = 0x05,
    /// Control.
    Ctl = 0x06,
    /// Control Mask.
    Mask = 0x07,
    /// Status.
    Stat = 0x08,
    /// Legacy output for serial daisy-chain.
    Lout = 0x09,
    /// Configuration Option 1.
    Cor1 = 0x0a,
    /// Configuration Option 2.
    Cor2 = 0x0b,
    /// Power-down Option register.
    PwrdnReg = 0x0c,
    /// Frame Length register.
    Flr = 0x0d,
    /// Product IDCODE.
    Idcode = 0x0e,
    /// Configuration Watchdog Timer.
    Cwdt = 0x0f,
    /// House Clean Option register.
    HcOptReg = 0x10,
    /// CSB output for parallel daisy-chaining.
    Csbo = 0x12,
    /// Power-up self test or loadable program address.
    General1 = 0x13,
    /// Power-up self test or loadable program address and new SPI opcode.
    General2 = 0x14,
    /// Golden bitstream address.
    General3 = 0x15,
    /// Golden bitstream address and new SPI  opcode.
    General4 = 0x16,
    /// User-defined register for fail-safe scheme.
    General5 = 0x17,
    /// Reboot mode.
    ModeReg = 0x18,
    /// GWE cycle during wake-up from suspend.
    PuGwe = 0x19,
    /// GTS cycle during wake-up from suspend.
    PuGts = 0x1a,
    /// Multi-frame write register.
    Mfwr = 0x1b,
    /// CCLK frequency select for master mode.
    CclkFreq = 0x1c,
    /// SEU frequency, enable and status.
    SeuOpt = 0x1d,
    /// Expected readback signature for SEU detection.
    ExpSign = 0x1e,
    /// Readback signature for readback command and SEU.
    RdbkSign = 0x1f,
    /// Boot History Register.
    Bootsts = 0x20,
    /// Mask pins for Multi-Pin Wake-Up.
    EyeMask = 0x21,
    /// Initial CBC Value Register.
    CbcReg = 0x22,
}

pub const SYNC0: u16 = 0xaa99;
pub const SYNC1: u16 = 0x5566;
pub const NOOP: u16 = 0x2000;

pub const fn type1(op: OpCode, addr: Addr, len: Words16<u8>) -> u16 {
    0b001 << 13 | (op as u16) << 11 | (addr as u16) << 5 | (len.0 as u16) & 0b11111
}

/// Must be followed by two 16-bit words representing the length
pub const fn type2(op: OpCode, addr: Addr) -> u16 {
    0b010 << 13 | (op as u16) << 11 | (addr as u16) << 5
}
