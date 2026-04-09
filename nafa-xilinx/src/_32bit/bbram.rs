use eyre::Result;
use nafa_io::{
    Command, Controller,
    devices::Xilinx32Family,
    units::{Bits, Bytes},
};

use crate::_32bit::{commands, commands::duplicated, crc::Crc};

pub async fn program_key(cont: &mut Controller, keys: &[[u8; 32]], dpa: Option<Dpa>) -> Result<()> {
    let info = match &cont.info().specific {
        nafa_io::devices::Specific::Xilinx32(info) => info,
        _ => panic!("xilinx bbram programming called with non-xilinx active device"),
    };
    assert_eq!(
        usize::from(info.slr),
        keys.len(),
        "must give one key per slr in device"
    );

    let has_crc = !matches!(info.family, Xilinx32Family::S7);

    let ctrl_word = ctrl_word(dpa, false);

    cont.run([
        Command::ir(duplicated(commands::JPROGRAM)),
        Command::ir(duplicated(commands::ISC_NOOP)),
    ])
    .await?;
    smol::Timer::after(std::time::Duration::from_millis(100)).await;

    let mut crc_correct = true;
    for key in keys {
        let key_chunks: &[[u8; 4]] = key.as_chunks().0;

        let enable = duplicated(commands::ISC_ENABLE);
        let program_key = duplicated(commands::XSC_PROG_SEC);
        let program = duplicated(commands::ISC_PROGRAM);

        #[rustfmt::skip]
        cont.run([
            Command::combined_ir_dr_tx_bits(enable, 0x15, Bits(5)),

            Command::ir(program_key),
            Command::dr_tx(&[0xff; 4]),
            Command::idle(Bytes(2)),

            Command::ir(program), Command::dr_tx(&ctrl_word.to_le_bytes()),
            Command::ir(program), Command::dr_tx(&swap_endian(key_chunks[0])),
            Command::ir(program), Command::dr_tx(&swap_endian(key_chunks[1])),
            Command::ir(program), Command::dr_tx(&swap_endian(key_chunks[2])),
            Command::ir(program), Command::dr_tx(&swap_endian(key_chunks[3])),
            Command::ir(program), Command::dr_tx(&swap_endian(key_chunks[4])),
            Command::ir(program), Command::dr_tx(&swap_endian(key_chunks[5])),
            Command::ir(program), Command::dr_tx(&swap_endian(key_chunks[6])),
            Command::ir(program), Command::dr_tx(&swap_endian(key_chunks[7])),
        ]).await?;

        if has_crc {
            crc_correct = check_crc(cont, ctrl_word, key).await?;
            if !crc_correct {
                break;
            }
        }
    }

    cont.run([Command::ir(duplicated(commands::ISC_DISABLE)), Command::dr_tx(&[0xff; 4])])
        .await?;

    if crc_correct {
        Ok(())
    } else {
        Err(eyre::eyre!("failed to verify crc"))
    }
}

async fn check_crc(cont: &mut Controller, ctrl_word: u32, key: &[u8; 32]) -> Result<bool> {
    let expected = crc(ctrl_word, key);
    let expected_bytes = expected.to_le_bytes();

    let program = [Command::ir(duplicated(commands::ISC_PROGRAM)), Command::dr_tx(&expected_bytes)];

    let readback = [Command::ir(duplicated(commands::ISC_READ)), Command::dr_rx(Bytes(5))];
    let readback = std::iter::repeat_n(readback, 10).flatten();

    let data = cont.run(program.into_iter().chain(readback)).await?;

    for chunk in data.as_chunks::<5>().0 {
        let read =
            u32::from_le_bytes(*chunk[0..4].as_array().unwrap()) >> 5 | u32::from(chunk[4]) << 27;
        tracing::info!(
            read = %nafa_io::Hex(read),
            "crc readback"
        );
        if read == expected {
            return Ok(true);
        }
    }

    Ok(false)
}

const fn swap_endian(x: [u8; 4]) -> [u8; 4] {
    u32::from_be_bytes(x).to_le_bytes()
}

#[derive(Clone, Copy, clap::ValueEnum)]
pub enum DpaMode {
    /// Normal operation. Decrement DPA counter upon failing to configure.
    Invalid,
    /// Decrement DPA counter on _any_ configuration, so the device has a fixed
    /// maximum number of configurations.
    All,
}

// Derive weirdness: `#[command(flatten)]` on an `Option<T>` will mark `T`'s
// fields as required. This gets around that: if any field is present, all
// fields are required. If no fields are present, none are required (and the
// option is `None`).
// See clap issue 5092
#[derive(Clone, Copy, clap::Args)]
#[group(requires_all = ["mode", "count"])]
pub struct Dpa {
    #[arg(long = "dpa-mode", required = false)]
    pub mode: DpaMode,
    /// Value to initialize the DPA counter to.
    ///
    /// After the counter reaches 0, the BBRAM key is cleared.
    #[arg(long = "dpa-count", required = false)]
    pub count: u8,
}

fn ctrl_word(dpa: Option<Dpa>, key_obfuscated: bool) -> u32 {
    const ENABLE: u32 = 2;
    const DISABLE: u32 = 1;
    fn shift(val: bool, amount: u32) -> u32 {
        if val {
            ENABLE << amount
        } else {
            DISABLE << amount
        }
    }

    let mode = shift(dpa.is_some_and(|d| matches!(d.mode, DpaMode::All)), 12);
    let enable = shift(dpa.is_some(), 14);
    let count = {
        let count = dpa.map_or(1, |dpa| dpa.count as u32);
        count << 16 | count << 24
    };
    let reserved = 0x0440;
    let black_key = shift(key_obfuscated, 8);
    ecc(mode | enable | count | reserved | black_key)
}

fn ecc(data: u32) -> u32 {
    const P0_MASK: u32 = 0x36AD555;
    const P1_MASK: u32 = 0x2D9B333;
    const P2_MASK: u32 = 0x1C78F0F;
    const P3_MASK: u32 = 0x03F80FF;
    const P4_MASK: u32 = 0x0007FFF;

    fn row(mut data: u32, mut mask: u32) -> u32 {
        let mut ret = 0;
        for _ in 0..26 {
            ret ^= (data & 1) & (mask & 1);
            data >>= 1;
            mask >>= 1;
        }
        ret
    }

    let p0 = row(data >> 6, P0_MASK);
    let p1 = row(data >> 6, P1_MASK);
    let p2 = row(data >> 6, P2_MASK);
    let p3 = row(data >> 6, P3_MASK);
    let p4 = row(data >> 6, P4_MASK);
    let p5 = {
        let mut value = data >> 6;
        let mut ret = p0 ^ p1 ^ p2 ^ p3 ^ p4;
        for _ in 0..26 {
            ret ^= value & 1;
            value >>= 1;
        }
        ret
    };

    data & 0xFFFF_FFC0 | p5 | p4 << 1 | p3 << 2 | p2 << 3 | p1 << 4 | p0 << 5
}

fn crc(ctrl: u32, key: &[u8; 32]) -> u32 {
    let (key, []) = key.as_chunks() else {
        unreachable!()
    };
    let mut crc = Crc::new(0);
    crc.update(9, ctrl);
    for (idx, chunk) in key.iter().enumerate() {
        crc.update(8 - (idx as u8), u32::from_be_bytes(*chunk));
    }
    crc.value()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbram_crc() {
        let ctrl = 0x0101555c;
        let key = &[
            0x12, 0x34, 0x56, 0x78, 0xAB, 0xCD, 0xDC, 0xBA, 0x12, 0x34, 0x56, 0x78, 0xAB, 0xCD,
            0xDC, 0xBA, 0x12, 0x34, 0x56, 0x78, 0xAB, 0xCD, 0xDC, 0xBA, 0x12, 0x34, 0x56, 0x78,
            0xAB, 0xCD, 0xDC, 0xBA,
        ];
        let expected = 0x29472b78;
        assert_eq!(crc(ctrl, key), expected);
    }

    #[test]
    fn test_bbram_ecc() {
        let ctrl_word = 0x01015540;
        assert_eq!(ecc(ctrl_word), 0x0101555c);

        let ctrl_word = super::ctrl_word(None, false);
        assert_eq!(ctrl_word, 0x0101555c);

        let dpa = Dpa {
            mode: DpaMode::Invalid,
            count: 6,
        };
        let ctrl_word = super::ctrl_word(Some(dpa), false);
        assert_eq!(ctrl_word, 0x06069542);

        let dpa = Dpa {
            mode: DpaMode::All,
            count: 24,
        };
        let ctrl_word = super::ctrl_word(Some(dpa), false);
        assert_eq!(ctrl_word, 0x1818a556);
    }
}
