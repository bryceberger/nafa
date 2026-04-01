use eyre::Result;
use facet::Facet;
use nafa_io::Controller;

use crate::{
    _32bit::info::{S7, UP, US},
    zynq_32::info::ZP,
};

pub mod _32bit;
pub mod zynq_32;

trait Read: Sized {
    async fn read(cont: &mut Controller) -> Result<Self>;
}

pub async fn read(cont: &mut Controller) -> Result<XilinxInfo> {
    use nafa_io::devices::{Specific as S, Xilinx32Family as F, Xilinx32Info as I};
    match cont.info().specific {
        S::Xilinx32(I { family: F::S7, .. }) => {
            Ok(XilinxInfo::S7(_32bit::info::S7::read(cont).await?))
        }
        S::Xilinx32(I { family: F::US, .. }) => {
            Ok(XilinxInfo::US(_32bit::info::US::read(cont).await?))
        }
        S::Xilinx32(I { family: F::UP, .. }) => {
            Ok(XilinxInfo::UP(_32bit::info::UP::read(cont).await?))
        }
        S::Xilinx32(I { family: F::ZP, .. }) => {
            Ok(XilinxInfo::ZP(zynq_32::info::ZP::read(cont).await?))
        }
        _ => Err(eyre::eyre!("unsupported device")),
    }
}

#[repr(C)]
#[derive(Facet)]
#[facet(tag = "family", content = "data")]
pub enum XilinxInfo {
    S7(S7),
    US(US),
    UP(UP),
    ZP(ZP),
}
