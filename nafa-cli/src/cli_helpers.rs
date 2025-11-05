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
