use eyre::Result;
use facet::Facet;
use nafa_io::Controller;

use super::commands;
use crate::{_32bit::read_jtag_simple, Read};

#[derive(Facet)]
pub struct PF {
    pub idcode: [u8; 4],
    pub udv: [u8; 4],
    pub silsig: [u8; 4],
    pub checksum: [u8; 2],

    pub read_design_info: [u8; 36],
    pub designver: [u8; 2],
    pub backlevel: [u8; 2],
    pub check_fabric_digest: [u8; 32],
    pub cc_digest: [u8; 32],
    pub snvm_digest: [u8; 32],
    pub ul_digest: [u8; 32],
    pub ukdigest0: [u8; 32],
    pub ukdigest1: [u8; 32],
    pub ukdigest2: [u8; 32],
    pub ukdigest3: [u8; 32],
    pub ukdigest4: [u8; 32],
    pub ukdigest5: [u8; 32],
    pub ukdigest6: [u8; 32],
    pub uperm_digest0: [u8; 32],
    pub sys_digest: [u8; 32],

    pub debug_info: [u8; 84],
    pub dsn: [u8; 16],
    pub query_security: [u8; 9],
    pub factory_integrity_bits: [u8; 32],
}

impl Read for PF {
    async fn read(cont: &mut Controller) -> Result<Self> {
        let jtag = read_pf_jtag_device(cont).await?;
        Ok(jtag)
    }
}

async fn read_pf_jtag_device(cont: &mut Controller) -> Result<PF> {
    Ok(PF {
        idcode: *read_jtag_simple(cont, commands::IDCODE, 4).await?,
        udv: [0; 4],
        silsig: *read_jtag_simple(cont, commands::SILSIG, 4).await?,
        checksum: [0; 2],
        read_design_info: [0; 36],
        designver: [0; 2],
        backlevel: [0; 2],
        check_fabric_digest: [0; 32],
        cc_digest: [0; 32],
        snvm_digest: [0; 32],
        ul_digest: [0; 32],
        ukdigest0: [0; 32],
        ukdigest1: [0; 32],
        ukdigest2: [0; 32],
        ukdigest3: [0; 32],
        ukdigest4: [0; 32],
        ukdigest5: [0; 32],
        ukdigest6: [0; 32],
        uperm_digest0: [0; 32],
        sys_digest: [0; 32],
        debug_info: [0; 84],
        dsn: [0; 16],
        query_security: [0; 9],
        factory_integrity_bits: [0; 32],
    })
}
