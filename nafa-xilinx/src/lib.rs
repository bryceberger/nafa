use eyre::Result;
use nafa_io::{Backend, Controller};

pub mod _32bit;
pub mod zynq_32;

pub trait Read: Sized {
    fn read(cont: &mut Controller<impl Backend>) -> impl Future<Output = Result<Self>>;
}

pub async fn read<R: Read>(cont: &mut Controller<impl Backend>) -> Result<R> {
    R::read(cont).await
}
