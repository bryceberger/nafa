use eyre::Result;
use nafa_io::{Backend, Controller, units::Words32};

use crate::_32bit::{
    read_register,
    registers::{Addr, OpCode, Type1},
};

#[derive(Default)]
struct Base {
    // efuse registers
    user1: [u8; 4],
    user2: [u8; 4],
    user3: [u8; 4],
    user4: [u8; 4],
}

#[derive(Default)]
pub struct S7 {
    base: Base,
    // bitstream registers
    ctl0: [u8; 4],
    ctl1: [u8; 4],
    // don't care about xadc regs (temperature, voltage, etc)
}

pub trait Reader {
    fn read<B: Backend>(cont: &mut Controller<B>) -> Result<Self>
    where
        Self: Sized;
}

impl Reader for S7 {
    fn read<B: Backend>(cont: &mut Controller<B>) -> Result<Self>
    where
        Self: Sized,
    {
        let mut ret = Self::default();

        for (ir, write_to) in [(0x00, &mut ret.base.user1), (0x00, &mut ret.base.user2)] {}

        for (reg, write_to) in [(Addr::Ctl0, &mut ret.ctl0), (Addr::Ctl1, &mut ret.ctl1)] {
            let out = read_register(cont, Type1::new(OpCode::Read, reg, Words32(1)))?;
            write_to.copy_from_slice(out);
        }

        Ok(ret)
    }
}
