use eyre::Result;
use facet::Facet;
use nafa_io::Controller;

use crate::_32bit::info::PF;

pub mod _32bit;

trait Read: Sized {
    async fn read(cont: &mut Controller) -> Result<Self>;
}

pub async fn read(cont: &mut Controller) -> Result<MicrochipInfo> {
    use nafa_io::devices::Specific as S;
    match cont.info().specific {
        S::Microchip => Ok(MicrochipInfo::PF(_32bit::info::PF::read(cont).await?)),
        _ => Err(eyre::eyre!("unsupported device")),
    }
}

#[repr(C)]
#[derive(Facet)]
#[facet(tag = "family", content = "data")]
pub enum MicrochipInfo {
    PF(PF),
}
