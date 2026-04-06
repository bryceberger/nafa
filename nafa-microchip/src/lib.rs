use eyre::Result;
use facet::Facet;
use nafa_io::Controller;

use crate::{
    _32bit::info::{S7, UP, US},
};

pub mod _32bit;

trait Read: Sized {
    async fn read(cont: &mut Controller) -> Result<Self>;
}

pub async fn read(cont: &mut Controller) -> Result<XilinxInfo> {
    use nafa_io::devices::{Specific as S, Xilinx32Family as F, Xilinx32Info as I};
    match cont.info().specific {
        S::Xilinx32(I { family: F::S7, .. }) => {
            Ok(XilinxInfo::S7(_32bit::info::S7::read(cont).await?))
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
}
