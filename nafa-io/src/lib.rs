mod backend;
pub mod cables;
pub mod controller;
pub mod devices;
pub mod ftdi;
pub mod jtag;
pub mod units;
pub mod usb_blaster;
mod utils;
pub mod xpc;

pub use crate::{
    backend::{Backend, Buffer, Data},
    controller::{Command, Controller, detect_chain},
    utils::{Hex, ShortHex, SpaceHex},
};

pub fn timeout<T>(duration: std::time::Duration, val: T) -> impl Future<Output = T> {
    use std::{task::Poll, time::Instant};

    let mut val = Some(val);
    let stop = Instant::now() + duration;

    smol::future::poll_fn(move |_| {
        if Instant::now() < stop {
            Poll::Pending
        } else {
            Poll::Ready(val.take().unwrap())
        }
    })
}
