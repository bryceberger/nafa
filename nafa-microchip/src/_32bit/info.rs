use eyre::Result;
use facet::Facet;
use nafa_io::Controller;
use zerocopy::FromBytes;

#[derive(FromBytes, Facet)]
#[repr(C)]
pub struct ReadDesignInfo {
    checksum: [u8; 2],
    designver: [u8; 2],
    backlevel: [u8; 2],
}

#[derive(FromBytes, Facet)]
#[repr(C)]
pub struct Digests {
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
}

#[derive(FromBytes)]
#[repr(C)]
pub struct DeviceIntegrityDSN {
    pub device_integrity_bits: [u8; 32],
    pub dsn: [u8; 16],
}

use crate::{
    _32bit::{
        debug_info, device_integrity_and_dsn, digests, idcode, query_security, read_design_info,
        silsig, udv,
    },
    Read,
};

#[derive(Facet)]
pub struct PF {
    pub idcode: [u8; 4],
    pub udv: [u8; 4],
    pub silsig: [u8; 4],

    pub read_design_info: [u8; 36],
    pub design_info: ReadDesignInfo,
    pub digests: Digests,

    pub debug_info: [u8; 84],
    pub dsn: [u8; 16],
    pub query_security: [u8; 9],
    pub device_integrity_bits: [u8; 32],
}

impl Read for PF {
    async fn read(cont: &mut Controller) -> Result<Self> {
        let jtag = read_pf_jtag_device(cont).await?;
        Ok(jtag)
    }
}

fn get_slice<const N: usize, T>(data: &[T], offset: usize) -> Option<&[T; N]> {
    data.get(offset..offset + N)?.as_array()
}

async fn read_pf_jtag_device(cont: &mut Controller) -> Result<PF> {
    let (buf, b) = cont.backend();
    let read_design_info = *get_slice(&read_design_info::<48>(b, buf).await?, 0).unwrap();
    let e1command = device_integrity_and_dsn::<32, 16>(b, buf).await?;
    Ok(PF {
        idcode: idcode(b, buf).await?,
        udv: udv(b, buf).await?,
        silsig: silsig(b, buf).await?,
        read_design_info,
        design_info: ReadDesignInfo {
            checksum: *get_slice(&read_design_info, 0).unwrap(),
            designver: *get_slice(&read_design_info, 32).unwrap(),
            backlevel: *get_slice(&read_design_info, 34).unwrap(),
        },
        digests: digests(b, buf).await?,
        debug_info: *get_slice(&debug_info::<96>(b, buf).await?, 0).unwrap(),
        dsn: e1command.dsn,
        query_security: *get_slice(&query_security::<16>(b, buf).await?, 0).unwrap(),
        device_integrity_bits: e1command.device_integrity_bits,
    })
}
