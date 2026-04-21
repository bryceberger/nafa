//! Utilities to read from Zynq devices.
//!
//! These are _almost_ the same as the rest of the devices. However, they have
//! an IRLEN of 12 instead of 6 --- there's a processor and FPGA stuck together
//! acting as a single device.

use nafa_io::{controller::TypedController, devices::Xilinx32Info};

pub mod actions;
mod commands;
mod io_utils;

pub type Controller<'a> = TypedController<'a, Xilinx32Info>;
