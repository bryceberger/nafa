use bitflags::bitflags;
use nafa_io::{controller::TypedController, devices::Xilinx16Info};

pub mod actions;
mod commands;
mod registers;

pub type Controller<'a> = TypedController<'a, Xilinx16Info>;

bitflags! {
    struct IRCapture: u8 {
        const DONE        = 0b000001;
        const INIT        = 0b000010;
        const ISC_ENABLED = 0b000100;
        const ISC_DONE    = 0b001000;
    }
}

const fn from_wire_order(x: [u8; 2]) -> u16 {
    u16::from_le_bytes(x).reverse_bits()
}

const fn to_wire_order(x: u16) -> [u8; 2] {
    x.reverse_bits().to_le_bytes()
}

fn bitstream_to_wire_order<const N: usize>(x: [u16; N]) -> [[u8; 2]; N] {
    x.map(to_wire_order)
}
