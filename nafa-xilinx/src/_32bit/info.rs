use eyre::Result;
use facet::Facet;
use nafa_io::{Backend, Controller, devices::DeviceInfo};

use super::{
    commands, read_device_register_word as device_register,
    read_jtag_register_sized as jtag_register, registers::Addr,
};
use crate::Read;

nestify::nest! {
#[derive(Facet)]
pub struct S7 {
    pub jtag: #[derive(Facet)] pub struct S7Jtag {
        pub device: #[derive(Facet)] pub struct S7JtagPerDevice {
            pub cntl: [u8; 2],
        },
        pub slrs: Vec<#[derive(Facet)] pub struct S7JtagPerSlr {
            pub idcode: [u8; 4],
            pub usercode: [u8; 4],
            pub fuse_dna: [u8; 8],
            pub fuse_key: [u8; 32],
            pub fuse_user: [u8; 4],
            pub user1: [u8; 4],
            pub user2: [u8; 4],
            pub user3: [u8; 4],
            pub user4: [u8; 4],
        }>,
    },
    pub registers: Registers,
}
}

nestify::nest! {
#[derive(Facet)]
pub struct US {
    pub jtag: #[derive(Facet)] pub struct USJtag {
        pub device: #[derive(Facet)] pub struct USJtagPerDevice {
            pub cntl: [u8; 3],
        },
        pub slrs: Vec<USJtagPerSlr>,
    },
    pub registers: Registers,
}
}

nestify::nest! {
#[derive(Facet)]
pub struct UP {
    pub jtag: #[derive(Facet)] pub struct UPJtag {
        pub device: #[derive(Facet)] pub struct UPJtagPerDevice {
            pub cntl: [u8; 4],
        },
        pub slrs: Vec<USJtagPerSlr>,
    },
    pub registers: Registers,
}
}

// shared with US/UP
#[derive(Facet)]
pub struct USJtagPerSlr {
    pub idcode: [u8; 4],
    pub usercode: [u8; 4],
    pub fuse_dna: [u8; 12],
    pub fuse_key: [u8; 32],
    pub fuse_user: [u8; 4],
    pub fuse_user_128: [u8; 16],
    pub fuse_rsa: [u8; 48],
    pub fuse_sec: [u8; 2],
    pub user1: [u8; 4],
    pub user2: [u8; 4],
    pub user3: [u8; 4],
    pub user4: [u8; 4],
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

fn get_num_slr(info: &DeviceInfo) -> u8 {
    match info.specific {
        nafa_io::devices::Specific::Xilinx32(nafa_io::devices::Xilinx32Info { slr, .. }) => slr,
        _ => panic!("should only ever call xilinx read with xilinx device active"),
    }
}

impl Read for S7 {
    async fn read(cont: &mut Controller<impl Backend>) -> Result<Self> {
        let num_slr = get_num_slr(cont.info());
        let jtag = S7Jtag {
            device: S7JtagPerDevice {
                cntl: *jtag_register(cont, 0, commands::FUSE_CNTL).await?,
            },
            slrs: read_slrs(num_slr, async |slr| {
                Ok(S7JtagPerSlr {
                    idcode: *jtag_register(cont, slr, commands::IDCODE).await?,
                    usercode: *jtag_register(cont, slr, commands::USERCODE).await?,
                    fuse_dna: *jtag_register(cont, slr, commands::FUSE_DNA).await?,
                    fuse_key: *jtag_register(cont, slr, commands::FUSE_KEY).await?,
                    fuse_user: *jtag_register(cont, slr, commands::FUSE_USER).await?,
                    user1: *jtag_register(cont, slr, commands::USER1).await?,
                    user2: *jtag_register(cont, slr, commands::USER2).await?,
                    user3: *jtag_register(cont, slr, commands::USER3).await?,
                    user4: *jtag_register(cont, slr, commands::USER4).await?,
                })
            })
            .await?,
        };
        let registers = read_registers(cont, num_slr).await?;
        Ok(S7 { jtag, registers })
    }
}

impl Read for US {
    async fn read(cont: &mut Controller<impl Backend>) -> Result<Self> {
        let num_slr = get_num_slr(cont.info());
        let jtag = USJtag {
            device: USJtagPerDevice {
                cntl: *jtag_register(cont, 0, commands::FUSE_CNTL).await?,
            },
            slrs: read_us_jtag_per_slr(cont, num_slr).await?,
        };
        let registers = read_registers(cont, num_slr).await?;
        Ok(US { jtag, registers })
    }
}

impl Read for UP {
    async fn read(cont: &mut Controller<impl Backend>) -> Result<Self> {
        let num_slr = get_num_slr(cont.info());
        let jtag = UPJtag {
            device: UPJtagPerDevice {
                cntl: *jtag_register(cont, 0, commands::FUSE_CNTL).await?,
            },
            slrs: read_us_jtag_per_slr(cont, num_slr).await?,
        };
        let registers = read_registers(cont, num_slr).await?;
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

async fn read_us_jtag_per_slr(
    cont: &mut Controller<impl Backend>,
    num_slr: u8,
) -> Result<Vec<USJtagPerSlr>, eyre::Error> {
    read_slrs(num_slr, async |slr| {
        Ok(USJtagPerSlr {
            idcode: *jtag_register(cont, slr, commands::IDCODE).await?,
            usercode: *jtag_register(cont, slr, commands::USERCODE).await?,
            fuse_dna: *jtag_register(cont, slr, commands::FUSE_DNA).await?,
            fuse_key: *jtag_register(cont, slr, commands::FUSE_KEY).await?,
            fuse_user: *jtag_register(cont, slr, commands::FUSE_USER).await?,
            fuse_user_128: *jtag_register(cont, slr, commands::FUSE_USER_128).await?,
            fuse_rsa: *jtag_register(cont, slr, commands::FUSE_RSA).await?,
            fuse_sec: *jtag_register(cont, slr, commands::FUSE_SEC).await?,
            user1: *jtag_register(cont, slr, commands::USER1).await?,
            user2: *jtag_register(cont, slr, commands::USER2).await?,
            user3: *jtag_register(cont, slr, commands::USER3).await?,
            user4: *jtag_register(cont, slr, commands::USER4).await?,
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
async fn read_registers(cont: &mut Controller<impl Backend>, num_slr: u8) -> Result<Registers> {
    let slrs = read_slrs(num_slr, async |slr| {
        Ok(RegistersPerSlr {
            ctl0: device_register(cont, slr, Addr::Ctl0).await?,
            stat: device_register(cont, slr, Addr::Stat).await?,
            cor0: device_register(cont, slr, Addr::Cor0).await?,
            idcode: device_register(cont, slr, Addr::Idcode).await?,
            axss: device_register(cont, slr, Addr::Axss).await?,
            cor1: device_register(cont, slr, Addr::Cor1).await?,
            wbstar: device_register(cont, slr, Addr::Wbstar).await?,
            timer: device_register(cont, slr, Addr::Timer).await?,
            bootsts: device_register(cont, slr, Addr::Bootsts).await?,
            ctl1: device_register(cont, slr, Addr::Ctl1).await?,
            bspi: device_register(cont, slr, Addr::Bspi).await?,
        })
    })
    .await?;
    Ok(Registers { slrs })
}
