use eyre::Result;
use facet::Facet;

use super::{
    commands, read_device_register_word as device_register,
    read_jtag_register_duplicated as jtag_duplicated, read_jtag_register_master as jtag_master,
    read_jtag_register_shifted as jtag_shifted, registers::Addr,
};
use crate::{Controller, Read};

#[derive(Facet)]
pub struct S7 {
    pub jtag: S7Jtag,
    pub registers: Registers,
}

#[derive(Facet)]
pub struct US {
    pub jtag: USJtag,
    pub registers: Registers,
}

#[derive(Facet)]
pub struct UP {
    pub jtag: USJtag,
    pub registers: Registers,
}

#[derive(Facet)]
pub struct S7Jtag {
    pub device: S7JtagPerDevice,
    pub slrs: Vec<S7JtagPerSlr>,
}

#[derive(Facet)]
pub struct S7JtagPerDevice {
    pub idcode: [u8; 4],
    pub usercode: [u8; 4],
    pub fuse_user: [u8; 4],
    pub user1: [u8; 4],
    pub user2: [u8; 4],
    pub user3: [u8; 4],
    pub user4: [u8; 4],
}

#[derive(Facet)]
pub struct S7JtagPerSlr {
    pub cntl: [u8; 2],
    pub fuse_dna: [u8; 8],
    pub fuse_key: [u8; 32],
}

// shared with US/UP
#[derive(Facet)]
pub struct USJtag {
    pub device: USJtagPerDevice,
    pub slrs: Vec<USJtagPerSlr>,
}

#[derive(Facet)]
pub struct USJtagPerDevice {
    pub idcode: [u8; 4],
    pub usercode: [u8; 4],
    pub fuse_user: [u8; 4],
    pub fuse_user_128: [u8; 4],
    pub user1: [u8; 4],
    pub user2: [u8; 4],
    pub user3: [u8; 4],
    pub user4: [u8; 4],
}

#[derive(Facet)]
pub struct USJtagPerSlr {
    pub cntl: [u8; 4],
    pub fuse_dna: [u8; 12],
    pub fuse_key: [u8; 32],
    pub fuse_rsa: [u8; 48],
    pub fuse_sec: [u8; 2],
}

#[derive(Facet)]
pub struct Registers {
    pub slrs: Vec<RegistersPerSlr>,
}

#[derive(Facet)]
pub struct RegistersPerSlr {
    pub ctl0: u32,
    pub stat: u32,
    pub cor0: u32,
    pub idcode: u32,
    pub axss: u32,
    pub cor1: u32,
    pub wbstar: u32,
    pub timer: u32,
    pub bootsts: u32,
    pub ctl1: u32,
    pub bspi: u32,
}

impl Read for S7 {
    async fn read(mut cont: Controller<'_>) -> Result<Self> {
        let jtag = S7Jtag {
            device: S7JtagPerDevice {
                idcode: *jtag_duplicated(cont.reborrow(), commands::IDCODE).await?,
                usercode: *jtag_master(cont.reborrow(), commands::USERCODE).await?,
                fuse_user: *jtag_master(cont.reborrow(), commands::FUSE_USER).await?,
                user1: *jtag_master(cont.reborrow(), commands::USER1).await?,
                user2: *jtag_master(cont.reborrow(), commands::USER2).await?,
                user3: *jtag_master(cont.reborrow(), commands::USER3).await?,
                user4: *jtag_master(cont.reborrow(), commands::USER4).await?,
            },
            slrs: read_slrs(cont.info().slr, async |slr| {
                Ok(S7JtagPerSlr {
                    cntl: *jtag_shifted(cont.reborrow(), slr, commands::FUSE_CNTL).await?,
                    fuse_dna: *jtag_shifted(cont.reborrow(), slr, commands::FUSE_DNA).await?,
                    fuse_key: *jtag_shifted(cont.reborrow(), slr, commands::FUSE_KEY).await?,
                })
            })
            .await?,
        };
        let registers = read_registers(cont).await?;
        Ok(S7 { jtag, registers })
    }
}

impl Read for US {
    async fn read(mut cont: Controller<'_>) -> Result<Self> {
        let jtag = USJtag {
            device: read_us_jtag_device(cont.reborrow()).await?,
            slrs: read_us_jtag_per_slr(cont.reborrow()).await?,
        };
        let registers = read_registers(cont.reborrow()).await?;
        Ok(US { jtag, registers })
    }
}

impl Read for UP {
    async fn read(mut cont: Controller<'_>) -> Result<Self> {
        let jtag = USJtag {
            device: read_us_jtag_device(cont.reborrow()).await?,
            slrs: read_us_jtag_per_slr(cont.reborrow()).await?,
        };
        let registers = read_registers(cont.reborrow()).await?;
        Ok(UP { jtag, registers })
    }
}

async fn read_slrs<T>(num_slr: u8, mut f: impl AsyncFnMut(u8) -> Result<T>) -> Result<Vec<T>> {
    let mut ret = Vec::with_capacity(num_slr.into());
    for slr in 0..num_slr {
        ret.push(f(slr).await?);
    }
    Ok(ret)
}

async fn read_us_jtag_device(mut cont: Controller<'_>) -> Result<USJtagPerDevice> {
    Ok(USJtagPerDevice {
        idcode: *jtag_duplicated(cont.reborrow(), commands::IDCODE).await?,
        usercode: *jtag_master(cont.reborrow(), commands::USERCODE).await?,
        fuse_user: *jtag_master(cont.reborrow(), commands::FUSE_USER).await?,
        fuse_user_128: *jtag_master(cont.reborrow(), commands::FUSE_USER).await?,
        user1: *jtag_master(cont.reborrow(), commands::USER1).await?,
        user2: *jtag_master(cont.reborrow(), commands::USER2).await?,
        user3: *jtag_master(cont.reborrow(), commands::USER3).await?,
        user4: *jtag_master(cont.reborrow(), commands::USER4).await?,
    })
}

async fn read_us_jtag_per_slr(mut cont: Controller<'_>) -> Result<Vec<USJtagPerSlr>, eyre::Error> {
    read_slrs(cont.info().slr, async |slr| {
        Ok(USJtagPerSlr {
            cntl: *jtag_shifted(cont.reborrow(), slr, commands::FUSE_CNTL).await?,
            fuse_dna: *jtag_shifted(cont.reborrow(), slr, commands::FUSE_DNA).await?,
            fuse_key: *jtag_shifted(cont.reborrow(), slr, commands::FUSE_KEY).await?,
            fuse_rsa: *jtag_shifted(cont.reborrow(), slr, commands::FUSE_RSA).await?,
            fuse_sec: *jtag_shifted(cont.reborrow(), slr, commands::FUSE_SEC).await?,
        })
    })
    .await
}

// This ends up doing something like:
//
// send: sync noop ctl0 noop
// read: ctl0
// flush
// send: sync noop stat noop
// read: stat
// flush
// ...
//
// Which is annoying. I would _like_ to do:
//
// send: sync noop ctl0 stat cor0 ...
// read: ctl0 stat cor0 ...
// flush
//
// However, attempting that (with or without noops between) seems to result in
// incorrect output.
// Specifically, it read `ctl0` twice, then zeroes for the rest (when attempted
// on an XC7A35T)
async fn read_registers(mut cont: Controller<'_>) -> Result<Registers> {
    let num_slr = cont.info().slr;
    let slrs = read_slrs(num_slr, async |slr| {
        Ok(RegistersPerSlr {
            ctl0: device_register(cont.reborrow(), slr, Addr::Ctl0).await?,
            stat: device_register(cont.reborrow(), slr, Addr::Stat).await?,
            cor0: device_register(cont.reborrow(), slr, Addr::Cor0).await?,
            idcode: device_register(cont.reborrow(), slr, Addr::Idcode).await?,
            axss: device_register(cont.reborrow(), slr, Addr::Axss).await?,
            cor1: device_register(cont.reborrow(), slr, Addr::Cor1).await?,
            wbstar: device_register(cont.reborrow(), slr, Addr::Wbstar).await?,
            timer: device_register(cont.reborrow(), slr, Addr::Timer).await?,
            bootsts: device_register(cont.reborrow(), slr, Addr::Bootsts).await?,
            ctl1: device_register(cont.reborrow(), slr, Addr::Ctl1).await?,
            bspi: device_register(cont.reborrow(), slr, Addr::Bspi).await?,
        })
    })
    .await?;
    Ok(Registers { slrs })
}
