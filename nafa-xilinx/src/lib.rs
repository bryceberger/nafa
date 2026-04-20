use eyre::Result;
use facet::Facet;
use nafa_io::{controller::TypedController, devices::Xilinx32Info};

use crate::{
    _32bit::info::{S7, UP, US},
    zynq_32::info::ZP,
};

pub mod _32bit;
pub mod zynq_32;

pub type Controller<'a> = TypedController<'a, Xilinx32Info>;

trait Read: Sized {
    async fn read(cont: Controller<'_>) -> Result<Self>;
}

pub async fn read(cont: Controller<'_>) -> Result<XilinxInfo> {
    use nafa_io::devices::Xilinx32Family as F;
    match cont.info().family {
        F::S7 => Ok(XilinxInfo::S7(_32bit::info::S7::read(cont).await?)),
        F::US => Ok(XilinxInfo::US(_32bit::info::US::read(cont).await?)),
        F::UP => Ok(XilinxInfo::UP(_32bit::info::UP::read(cont).await?)),
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
