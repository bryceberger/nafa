use eyre::Result;
use facet::Facet;
use nafa_io::Controller;

use super::{
    read_device_register_word as device_register, read_jtag_register_sized as jtag_register,
};
use crate::{
    _32bit::{
        info::{Registers, RegistersPerSlr},
        registers::Addr,
    },
    Read,
    zynq_32::commands,
};

#[derive(Facet)]
pub struct ZP {
    pub jtag: ZPJtag,
    pub registers: Registers,
}

#[derive(Facet)]
pub struct ZPJtag {
    pub idcode_ps: [u8; 4],
    pub idcode_pl: [u8; 4],
    pub idcode_pspl: [u8; 8],
    pub usercode: [u8; 4],
    pub jtag_status: [u8; 4],
    pub jstatus: [u8; 4],
    pub xsc_dna: [u8; 12],
    pub fuse_key: [u8; 32],
    pub fuse_dna: [u8; 8],
    pub fuse_cntl: [u8; 4],
    pub fuse_user_ps: [u8; 4],
    pub user1: [u8; 4],
    pub user2: [u8; 4],
    pub user3: [u8; 4],
    pub user4: [u8; 4],
    // 121 bits
    pub error_status: [u8; 16],
}

impl Read for ZP {
    async fn read(cont: &mut Controller) -> Result<Self> {
        let jtag = ZPJtag {
            idcode_ps: *jtag_register(cont, commands::IDCODE).await?,
            idcode_pl: *jtag_register(cont, commands::IDCODE_PL).await?,
            idcode_pspl: *jtag_register(cont, commands::IDCODE_PSPL).await?,
            usercode: *jtag_register(cont, commands::USERCODE).await?,
            jtag_status: *jtag_register(cont, commands::JTAG_STATUS).await?,
            jstatus: *jtag_register(cont, commands::JSTATUS).await?,
            xsc_dna: *jtag_register(cont, commands::XSC_DNA).await?,
            fuse_key: *jtag_register(cont, commands::FUSE_KEY).await?,
            fuse_dna: *jtag_register(cont, commands::FUSE_DNA).await?,
            fuse_cntl: *jtag_register(cont, commands::FUSE_CNTL).await?,
            fuse_user_ps: *jtag_register(cont, commands::FUSE_USER_PS).await?,
            user1: *jtag_register(cont, commands::USER1).await?,
            user2: *jtag_register(cont, commands::USER2).await?,
            user3: *jtag_register(cont, commands::USER3).await?,
            user4: *jtag_register(cont, commands::USER4).await?,
            error_status: *jtag_register(cont, commands::ERROR_STATUS).await?,
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
