use std::iter::Fuse;

use eyre::{Ok, Result};
use nafa_io::{Backend, Command, Controller, units::Words32};

use crate::_32bit::{
    commands::{
        ERROR_STATUS, FUSE_CNTL, FUSE_DNA, FUSE_KEY, FUSE_USER, IP_DISABLE, JTAG_CTRL, JTAG_STATUS,
        USER1, USER2, USER3, USER4, USERCODE,
    },
    read_register,
    registers::{Addr, OpCode, Type1},
};

// Typical fuses in AMD-Xilinx FPGAs that have fuses
#[derive(Default)]
struct Fuses {
    // efuse registers
    user1: [u8; 4],
    user2: [u8; 4],
    user3: [u8; 4],
    user4: [u8; 4],

    cntl: [u8; 4],
    // fuse_sec: [u8; 4],
    // fuse_user128: [u8; 16],
    fuse_user: [u8; 4],
    // DNA is a vec because it should be there for every AMD FPGA, but has varying length
    dna: Vec<u8>,
    key: [u8; 4],
    usercode: [u8; 4],
}

// Devices that do not have any fuses. These are technically volatile but are registers of the same name and JTAG command (typically)
#[derive(Default)]
struct Fuseless {
    // efuse registers
    user1: [u8; 4],
    user2: [u8; 4],
    usercode: [u8; 4],
}

#[derive(Default)]
struct FusesSoc {
    // efuse registers
    user1: [u8; 4],
    user2: [u8; 4],
    user3: [u8; 4],
    user4: [u8; 4],

    // cntl: [u8; 4],
    // fuse_sec: [u8; 4],
    // fuse_user128: [u8; 16],
    // fuse_user: [u8; 4],
    dna: Vec<u8>,
    // key: [u8; 4],
    usercode: [u8; 4],
}

// Status Register struct of bitstream registers to be used within each of the family structs
#[derive(Default)]
struct StatReg {
    bootsts: [u8; 4],
    config_sts: [u8; 4],
    cor0: [u8; 4],
    ctl0: [u8; 4],
    ctl1: [u8; 4],
    // timer: [u8; 4],
    // wbstar: [u8; 4],
    // bspi: [u8; 4],
    // jtag_err: [u8; 15], ZynqUS+ PS
    // pstap_stat: [u8; 4], ZynqUS+ PS
    // ip_disabled: [u8; 4], ZynqUS+ PS
}

#[derive(Default)]
pub struct S2 {
    fuseless: Fuseless,
    // other bitstream registers
    ctl: [u8; 4],
    config_sts: [u8; 4],
    cor: [u8; 4],
    // don't care about xadc regs (temperature, voltage, etc)
}

#[derive(Default)]
pub struct V4 {
    fuseless: Fuseless,
    user3: [u8; 4],
    user4: [u8; 4],
    // other bitstream registers
    ctl0: [u8; 4],
    // don't care about xadc regs (temperature, voltage, etc)
}

#[derive(Default)]
pub struct V5 {
    // Fuseless volatile JTAG registers same as eFUSE on new parts
    fuseless: Fuseless,
    user3: [u8; 4],
    user4: [u8; 4],
    // bitstream registers
    status: StatReg,
    wbstar: [u8; 4],
    // don't care about xadc regs (temperature, voltage, etc)
}

#[derive(Default)]
pub struct V6 {
    fuse: Fuses,
    status: StatReg,
    // other bitstream registers
    timer: [u8; 4],
    wbstar: [u8; 4],
    bspi: [u8; 4],
    // don't care about xadc regs (temperature, voltage, etc)
}

#[derive(Default)]
pub struct S7 {
    fuse: Fuses,
    status: StatReg,
    // other bitstream registers
    timer: [u8; 4],
    wbstar: [u8; 4],
    bspi: [u8; 4],
    // don't care about xadc regs (temperature, voltage, etc)
}

#[derive(Default)]
pub struct Z7 {
    fuse: Fuses,
    status: StatReg,
    // other bitstream registers
    timer: [u8; 4],
    wbstar: [u8; 4],
    bspi: [u8; 4],
    // don't care about xadc regs (temperature, voltage, etc)
}

// This is the storage for US and US+, but they have some different
// register outputs. Separate because of this?
#[derive(Default)]
pub struct US {
    fuse: Fuses,
    status: StatReg,
    // other bitstream registers
    timer: [u8; 4],
    wbstar: [u8; 4],
    bspi: [u8; 4],
    // don't care about xadc regs (temperature, voltage, etc)
}

#[derive(Default)]
pub struct ZUS {
    fuse: FusesSoc,
    status: StatReg,
    // other bitstream registers
    timer: [u8; 4],
    wbstar: [u8; 4],
    bspi: [u8; 4],
    // Registers read from the JTAG directly that are status registers
    // from the processor (PS) portion of the Zynq
    jtag_err: [u8; 15],  // ZynqUS+ PS
    pstap_stat: [u8; 4], // ZynqUS+ PS
    ip_disabled: [u8; 4], // ZynqUS+ PS
                         // don't care about xadc regs (temperature, voltage, etc)
}

pub trait Reader {
    fn read<B: Backend>(cont: &mut Controller<B>) -> Result<Self>
    where
        Self: Sized;
}

impl Reader for StatReg {
    fn read<B: Backend>(cont: &mut Controller<B>) -> Result<Self>
    where
        Self: Sized,
    {
        let mut ret = Self::default();

        for (reg, write_to) in [
            (Addr::Bootsts, &mut ret.bootsts),
            (Addr::Stat, &mut ret.config_sts),
            (Addr::Cor0, &mut ret.cor0),
            (Addr::Ctl0, &mut ret.ctl0),
            (Addr::Ctl1, &mut ret.ctl1),
        ] {
            let out = read_register(cont, Type1::new(OpCode::Read, reg, Words32(1)))?;
            write_to.copy_from_slice(out);
        }

        Ok(ret)
    }
}

impl Reader for Fuses {
    fn read<B: Backend>(cont: &mut Controller<B>) -> Result<Self>
    where
        Self: Sized,
    {
        let mut ret = Self::default();

        for (ir, write_to) in [
            (USER1, &mut ret.user1),
            (USER2, &mut ret.user2),
            (USER3, &mut ret.user3),
            (USER4, &mut ret.user4),
            (FUSE_CNTL, &mut ret.cntl),
            (FUSE_USER, &mut ret.fuse_user),
            // (FUSE_DNA, &mut ret.dna), // TODO DNA is variable len so shouldn't actually do it here?
            (FUSE_KEY, &mut ret.key),
            (USERCODE, &mut ret.usercode),
        ] {
            let _ = cont.run([Command::ir(ir as _), Command::dr_tx(write_to)]);
        }

        Ok(ret)
    }
}

impl Reader for Fuseless {
    fn read<B: Backend>(cont: &mut Controller<B>) -> Result<Self>
    where
        Self: Sized,
    {
        let mut ret = Self::default();

        for (ir, write_to) in
            [(USER1, &mut ret.user1), (USER2, &mut ret.user2), (USERCODE, &mut ret.usercode)]
        {
            let _ = cont.run([Command::ir(ir as _), Command::dr_tx(write_to)]);
        }

        Ok(ret)
    }
}

impl Reader for FusesSoc {
    fn read<B: Backend>(cont: &mut Controller<B>) -> Result<Self>
    where
        Self: Sized,
    {
        let mut ret = Self::default();

        for (ir, write_to) in [
            (USER1, &mut ret.user1),
            (USER2, &mut ret.user2),
            (USER3, &mut ret.user3),
            (USER4, &mut ret.user4),
            // (FUSE_DNA, &mut ret.dna), // TODO DNA is variable len so shouldn't actually do it here?
            (USERCODE, &mut ret.usercode),
        ] {
            let _ = cont.run([Command::ir(ir as _), Command::dr_tx(write_to)]);
        }

        Ok(ret)
    }
}

impl Reader for S2 {
    fn read<B: Backend>(cont: &mut Controller<B>) -> Result<Self>
    where
        Self: Sized,
    {
        let mut ret = Self::default();

        // fuseless.read(cont) // TODO: How read/use a Reader trait?

        for (reg, write_to) in [
            (Addr::Ctl0, &mut ret.ctl),
            (Addr::Stat, &mut ret.config_sts),
            (Addr::Cor0, &mut ret.cor),
        ] {
            let out = read_register(cont, Type1::new(OpCode::Read, reg, Words32(1)))?;
            write_to.copy_from_slice(out);
        }

        Ok(ret)
    }
}

impl Reader for V4 {
    fn read<B: Backend>(cont: &mut Controller<B>) -> Result<Self>
    where
        Self: Sized,
    {
        let mut ret = Self::default();

        // fuseless.read(cont) // TODO: How read/use a Reader trait?

        let out = read_register(cont, Type1::new(OpCode::Read, Addr::Ctl0, Words32(1)))?;
        ret.ctl0.copy_from_slice(out);

        Ok(ret)
    }
}

impl Reader for V5 {
    fn read<B: Backend>(cont: &mut Controller<B>) -> Result<Self>
    where
        Self: Sized,
    {
        let mut ret = Self::default();

        // fuseless.read(cont) // TODO: How read/use a Reader trait?
        // status.read(cont)

        for (ir, write_to) in [(USER3, &mut ret.user3), (USER4, &mut ret.user4)] {
            let _ = cont.run([Command::ir(ir as _), Command::dr_tx(write_to)]);
        }

        let out = read_register(cont, Type1::new(OpCode::Read, Addr::Wbstar, Words32(1)))?;
        ret.wbstar.copy_from_slice(out);

        Ok(ret)
    }
}

impl Reader for S7 {
    fn read<B: Backend>(cont: &mut Controller<B>) -> Result<Self>
    where
        Self: Sized,
    {
        let mut ret = Self::default();

        // fuse.read(cont) // TODO: How read/use a Reader trait?
        // status.read(cont)

        for (reg, write_to) in [
            (Addr::Timer, &mut ret.timer),
            (Addr::Wbstar, &mut ret.wbstar),
            (Addr::Bspi, &mut ret.bspi),
        ] {
            let out = read_register(cont, Type1::new(OpCode::Read, reg, Words32(1)))?;
            write_to.copy_from_slice(out);
        }

        Ok(ret)
    }
}

impl Reader for Z7 {
    fn read<B: Backend>(cont: &mut Controller<B>) -> Result<Self>
    where
        Self: Sized,
    {
        let mut ret = Self::default();

        // fuse.read(cont) // TODO: How read/use a Reader trait?
        // status.read(cont)

        for (reg, write_to) in [
            (Addr::Timer, &mut ret.timer),
            (Addr::Wbstar, &mut ret.wbstar),
            (Addr::Bspi, &mut ret.bspi),
        ] {
            let out = read_register(cont, Type1::new(OpCode::Read, reg, Words32(1)))?;
            write_to.copy_from_slice(out);
        }

        Ok(ret)
    }
}

impl Reader for US {
    fn read<B: Backend>(cont: &mut Controller<B>) -> Result<Self>
    where
        Self: Sized,
    {
        let mut ret = Self::default();

        // fuse.read(cont) // TODO: How read/use a Reader trait?
        // status.read(cont)
        // read(fuse)

        for (reg, write_to) in [
            (Addr::Timer, &mut ret.timer),
            (Addr::Wbstar, &mut ret.wbstar),
            (Addr::Bspi, &mut ret.bspi),
        ] {
            let out = read_register(cont, Type1::new(OpCode::Read, reg, Words32(1)))?;
            write_to.copy_from_slice(out);
        }

        Ok(ret)
    }
}

impl Reader for ZUS {
    fn read<B: Backend>(cont: &mut Controller<B>) -> Result<Self>
    where
        Self: Sized,
    {
        let mut ret = Self::default();

        // fuse.read(cont) // TODO: How read/use a Reader trait?
        // status.read(cont)

        for (ir, write_to) in
            [(JTAG_STATUS, &mut ret.pstap_stat), (IP_DISABLE, &mut ret.ip_disabled)]
        {
            let _ = cont.run([Command::ir(ir as _), Command::dr_tx(write_to)]);
        }

        let out = cont.run([Command::ir(ERROR_STATUS as _), Command::dr_tx(&ret.jtag_err)]);

        for (reg, write_to) in [
            (Addr::Timer, &mut ret.timer),
            (Addr::Wbstar, &mut ret.wbstar),
            (Addr::Bspi, &mut ret.bspi),
        ] {
            let out = read_register(cont, Type1::new(OpCode::Read, reg, Words32(1)))?;
            write_to.copy_from_slice(out);
        }

        Ok(ret)
    }
}
