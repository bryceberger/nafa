use bitflags::bitflags;
use nafa_io::{controller::TypedController, devices::Xilinx32Info};

pub mod actions;
pub(crate) mod commands;
mod crc;
pub mod drp;
mod io_utils;
pub mod nky;
pub(crate) mod registers;

pub type Controller<'a> = TypedController<'a, Xilinx32Info>;

bitflags! {
    // note: these bits are reversed from what you might expect reading a BSDL
    // file. This is due to bit 0 being shifted out first, thus ending up on the
    // left-most, thus being the MSB instead of LSB.
    struct IRCapture: u8 {
        const DONE        = 0b000001;
        const INIT        = 0b000010;
        const ISC_ENABLED = 0b000100;
        const ISC_DONE    = 0b001000;
    }
}

pub const fn to_wire_order(x: u32) -> [u8; 4] {
    x.reverse_bits().to_le_bytes()
}

pub const fn from_wire_order(x: [u8; 4]) -> u32 {
    u32::from_le_bytes(x).reverse_bits()
}

pub(crate) fn bitstream_to_wire_order<const N: usize>(x: [u32; N]) -> [[u8; 4]; N] {
    x.map(to_wire_order)
}
