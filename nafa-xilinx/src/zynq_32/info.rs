use eyre::Result;
use facet::Facet;
use nafa_io::{Backend, Controller};

use super::{
    read_device_register_word as device_register, read_jtag_register_sized as jtag_register,
};
use crate::{
    _32bit::{
        commands,
        info::{Registers, RegistersPerSlr, UPJtag, UPJtagPerDevice, USJtagPerSlr},
        registers::Addr,
    },
    Read,
};

#[derive(Facet)]
pub struct ZP {
    pub jtag: UPJtag,
    pub registers: Registers,
}

impl Read for ZP {
    async fn read(cont: &mut Controller<impl Backend>) -> Result<Self> {
        let jtag = UPJtag {
            device: UPJtagPerDevice {
                cntl: *jtag_register(cont, commands::FUSE_CNTL).await?,
            },
            slrs: vec![USJtagPerSlr {
                idcode: *jtag_register(cont, commands::IDCODE).await?,
                usercode: *jtag_register(cont, commands::USERCODE).await?,
                fuse_dna: *jtag_register(cont, commands::FUSE_DNA).await?,
                fuse_key: *jtag_register(cont, commands::FUSE_KEY).await?,
                fuse_user: *jtag_register(cont, commands::FUSE_USER).await?,
                fuse_user_128: *jtag_register(cont, commands::FUSE_USER_128).await?,
                fuse_rsa: *jtag_register(cont, commands::FUSE_RSA).await?,
                fuse_sec: *jtag_register(cont, commands::FUSE_SEC).await?,
                user1: *jtag_register(cont, commands::USER1).await?,
                user2: *jtag_register(cont, commands::USER2).await?,
                user3: *jtag_register(cont, commands::USER3).await?,
                user4: *jtag_register(cont, commands::USER4).await?,
            }],
        };
        let registers = Registers {
            slrs: vec![RegistersPerSlr {
                ctl0: device_register(cont, Addr::Ctl0).await?,
                stat: device_register(cont, Addr::Stat).await?,
                cor0: device_register(cont, Addr::Cor0).await?,
                idcode: device_register(cont, Addr::Idcode).await?,
                axss: device_register(cont, Addr::Axss).await?,
                cor1: device_register(cont, Addr::Cor1).await?,
                wbstar: device_register(cont, Addr::Wbstar).await?,
                timer: device_register(cont, Addr::Timer).await?,
                bootsts: device_register(cont, Addr::Bootsts).await?,
                ctl1: device_register(cont, Addr::Ctl1).await?,
                bspi: device_register(cont, Addr::Bspi).await?,
            }],
        };
        Ok(Self { jtag, registers })
    }
}
