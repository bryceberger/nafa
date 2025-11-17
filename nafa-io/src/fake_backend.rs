use eyre::Result;

use crate::{Backend, Buffer, Data, jtag, units::Bits};

pub struct Cycle {
    pub tms: bool,
    pub tdi: bool,
    pub tdo: bool,
}

pub struct FakeBackend {
    cycles: Vec<Cycle>,
    idcode: u32,
    read_len: usize,
}

fn write_waveform(
    bits: impl Iterator<Item = bool>,
    start: bool,
    top: &mut String,
    bot: &mut String,
) {
    let mut prev = start;
    for b in bits {
        match (prev, b) {
            (true, true) => {
                top.push_str("──");
                bot.push_str("  ");
            }
            (true, false) => {
                top.push_str("─┐");
                bot.push_str(" └");
            }
            (false, true) => {
                top.push_str(" ┌");
                bot.push_str("─┘");
            }
            (false, false) => {
                top.push_str("  ");
                bot.push_str("──");
            }
        }
        prev = b;
    }
}

fn write_clk(len: usize, top: &mut String, bot: &mut String) {
    top.extend(std::iter::repeat_n("┌┐", len));
    bot.extend(std::iter::repeat_n("┘└", len));
}

fn write_cycles(cycles: &[Cycle]) -> String {
    let header = "┌Signals┐┌Waves";
    let footer = "└───────┘└─────";
    let mut tck_top = String::from("│tck    ││");
    let mut tck_bot = String::from("│       ││");
    write_clk(cycles.len(), &mut tck_top, &mut tck_bot);
    let mut tms_top = String::from("│tms    ││");
    let mut tms_bot = String::from("│       ││");
    write_waveform(
        cycles.iter().map(|c| c.tms),
        false,
        &mut tms_top,
        &mut tms_bot,
    );
    let mut tdi_top = String::from("│tdi    ││");
    let mut tdi_bot = String::from("│       ││");
    write_waveform(
        cycles.iter().map(|c| c.tdi),
        false,
        &mut tdi_top,
        &mut tdi_bot,
    );
    let mut tdo_top = String::from("│tdo    ││");
    let mut tdo_bot = String::from("│       ││");
    write_waveform(
        cycles.iter().map(|c| c.tdo),
        false,
        &mut tdo_top,
        &mut tdo_bot,
    );
    let wave_width = cycles.len() * 2;
    let mut ret = String::from(header);
    ret.extend(std::iter::repeat_n('─', wave_width.saturating_sub(5)));
    ret.push_str("┐\n");
    for buf in [&tck_top, &tck_bot, &tms_top, &tms_bot, &tdi_top, &tdi_bot, &tdo_top, &tdo_bot] {
        ret.push_str(buf);
        ret.push_str("│\n");
    }
    ret.push_str(footer);
    ret.extend(std::iter::repeat_n('─', wave_width.saturating_sub(5)));
    ret.push_str("┘\n");
    ret
}

impl FakeBackend {
    pub fn new(idcode: u32) -> Self {
        Self {
            cycles: Vec::new(),
            idcode,
            read_len: 0,
        }
    }

    fn add_bit(&mut self, tms: bool, tdi: bool, tdo: bool) {
        self.cycles.push(Cycle { tms, tdi, tdo })
    }

    pub fn consume_waveform(&mut self) -> String {
        let ret = write_cycles(&self.cycles);
        self.cycles.clear();
        ret
    }
}

impl Backend for FakeBackend {
    fn tms(&mut self, _buf: &mut dyn Buffer, path: jtag::Path) -> Result<()> {
        let tdi = true;
        let tdo = false;
        for tms in path {
            self.add_bit(tms, tdi, tdo);
        }
        Ok(())
    }

    fn bytes(
        &mut self,
        buf: &mut dyn Buffer,
        before: Option<jtag::Path>,
        data: Data<'_>,
        after: Option<jtag::Path>,
    ) -> Result<()> {
        if let Some(path) = before {
            self.tms(buf, path)?;
        }

        let tms = false;
        let mut last_tdi = true;
        let mut last_tdo = false;
        match data {
            Data::Tx(tdi) | Data::TxRx(tdi) => {
                let tdo = matches!(data, Data::TxRx(_));
                for (idx, byte) in tdi.iter().copied().enumerate() {
                    self.add_bit(tms, byte & 1 != 0, tdo);
                    self.add_bit(tms, byte >> 1 & 1 != 0, tdo);
                    self.add_bit(tms, byte >> 2 & 1 != 0, tdo);
                    self.add_bit(tms, byte >> 3 & 1 != 0, tdo);
                    self.add_bit(tms, byte >> 4 & 1 != 0, tdo);
                    self.add_bit(tms, byte >> 5 & 1 != 0, tdo);
                    self.add_bit(tms, byte >> 6 & 1 != 0, tdo);
                    if after.is_some() && idx == tdi.len() - 1 {
                        last_tdi = byte >> 7 & 1 != 0;
                        last_tdo = tdo;
                    } else {
                        self.add_bit(tms, byte >> 7 & 1 != 0, tdo);
                    }
                }

                if tdo {
                    self.read_len += tdi.len();
                }
            }
            Data::Rx(len) | Data::ConstantTx(_, len) => {
                let tdi = match data {
                    Data::ConstantTx(tdi, _) => tdi,
                    _ => true,
                };
                let tdo = matches!(data, Data::Rx(_));
                for idx in 0..len.0 {
                    self.add_bit(tms, tdi, tdo);
                    self.add_bit(tms, tdi, tdo);
                    self.add_bit(tms, tdi, tdo);
                    self.add_bit(tms, tdi, tdo);
                    self.add_bit(tms, tdi, tdo);
                    self.add_bit(tms, tdi, tdo);
                    self.add_bit(tms, tdi, tdo);
                    if after.is_some() && idx == len.0 - 1 {
                        last_tdo = tdo;
                    } else {
                        self.add_bit(tms, tdi, tdo);
                    }
                }

                if tdo {
                    self.read_len += len.0;
                }
            }
        }

        if let Some(path) = after {
            let mut it = path.into_iter();
            if let Some(tms) = it.next() {
                self.add_bit(tms, last_tdi, last_tdo);
            }
            for tms in it {
                self.add_bit(tms, true, false);
            }
        }

        Ok(())
    }

    fn bits(
        &mut self,
        buf: &mut dyn Buffer,
        before: Option<jtag::Path>,
        mut data: u32,
        len: Bits<u8>,
        after: Option<jtag::Path>,
    ) -> Result<()> {
        if let Some(path) = before {
            self.tms(buf, path)?;
        }

        let len = match after {
            Some(_) => len.0 - 1,
            None => len.0,
        };

        let tms = false;
        for _ in 0..len {
            self.add_bit(tms, data & 1 == 1, false);
            data >>= 1;
        }

        if let Some(path) = after {
            let mut it = path.into_iter();
            if let Some(tms) = it.next() {
                self.add_bit(tms, data & 1 == 1, false);
            }
            for tms in it {
                self.add_bit(tms, true, false);
            }
        }

        Ok(())
    }

    fn flush(&mut self, buf: &mut dyn Buffer) -> Result<()> {
        let buf = buf.extend(self.read_len);
        self.read_len = 0;
        buf.fill(0xff);
        if buf.len() >= 4 {
            buf[..4].copy_from_slice(&self.idcode.to_le_bytes()[..]);
        }
        Ok(())
    }
}
