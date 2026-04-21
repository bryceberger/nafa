use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use color_eyre::eyre::OptionExt;

#[derive(Debug, Clone, Copy)]
pub struct UsbAddr {
    pub vid: u16,
    pub pid: u16,
}

impl FromStr for UsbAddr {
    type Err = color_eyre::eyre::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let (vid, pid) = s.split_once(':').ok_or_eyre("no ':'")?;
        let vid = u16::from_str_radix(vid, 16)?;
        let pid = u16::from_str_radix(pid, 16)?;
        Ok(Self { vid, pid })
    }
}

impl Display for UsbAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04X}:{:04X}", self.vid, self.pid)
    }
}

#[repr(transparent)]
#[derive(Clone)]
pub struct Hex<const N: usize>(pub [u8; N]);

impl<const N: usize> FromStr for Hex<N> {
    type Err = hex::FromHexError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut ret = [0; N];
        hex::decode_to_slice(s, &mut ret)?;
        Ok(Self(ret))
    }
}

pub fn as_millis(d: std::time::Duration) -> f32 {
    const NANOS_PER_MILLI: u32 = 1_000_000;
    (d.as_nanos() as f32) / (NANOS_PER_MILLI as f32)
}
