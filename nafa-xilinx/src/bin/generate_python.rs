use facet::Facet;
use nafa_xilinx::{
    _32bit::info::{S7, UP, US},
    zynq_32::info::ZP,
};

fn main() {
    #[derive(Facet)]
    struct _Ignore {
        s7: S7,
        up: UP,
        us: US,
        zp: ZP,
    }
    println!("{}", facet_python::to_python::<_Ignore>(true));
}
