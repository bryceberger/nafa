use eyre::{OptionExt, Result};
use memchr::memmem;
use nafa_io::units::Words32;

use crate::_32bit::registers::{Addr, OpCode, Type1};

pub type Bitstream<'d> = Vec<SsitSection<'d>>;

/// Generally, an SSIT bitstream looks like:
///
/// ```text
/// (slr) command
/// -------------
///   2   businit
///   2   sync
///   2   data...
///   2   sync
///   2   cmd (shutdown)
///   2   cmd (reset crc)
///   2   ssit
///   1   ├── businit
///   1   ├── sync
///   1   ├── data...
///   1   ├── sync
///   1   ├── cmd (shutdown)
///   1   ├── cmd (reset crc)
///   1   ├── ssit
///   0   │   ├── businit
///   0   │   ├── sync
///   0   │   ├── data...
///   0   │   ├── cmd (desync)
///   0   │   └── cmd (start)
///   1   ├── cmd (desync)
///   1   └── cmd (start)
///   2   cmd (desync)
///   2   cmd (start)
/// ```
///
/// When programming, you send the first set of 2, then first 1, then all 0,
/// then second 1, then second 2. Essentially, sending the bitstream in the
/// order it appears in the file, walking down then back up the SLRs.
///
/// This translates into the [`Bitstream`] type alias, with `bit[0]` being the
/// outermost and `bit[bit.len()-1]` the innermost. Send `bit[n].pre`, then
/// `bit[n+1..]`, then `bit[n].post` (if non-empty).
#[derive(Debug, PartialEq, Eq)]
pub struct SsitSection<'d> {
    pub pre: &'d [u8],
    pub post: &'d [u8],
}

pub fn parse(data: &[u8]) -> Result<Bitstream<'_>> {
    const SYNC: [u8; 4] = Type1::SYNC.reverse_bits().to_le_bytes();

    let original = data.as_ptr();

    let first_sync = memmem::find(data, &SYNC).ok_or_eyre("no sync")?;
    let data = &data[first_sync..];
    let (mut data, []) = data.as_chunks() else {
        return Err(eyre::eyre!("non-word-aligned after sync"));
    };

    let mut ret = Vec::new();
    let mut prev = ret.push_mut(SsitSection {
        pre: data.as_flattened(),
        post: &[],
    });

    while let Some(section) = find_ssit(original, data)? {
        let offset_start = section.as_ptr().addr() - prev.pre.as_ptr().addr();
        let offset_end = offset_start + section.as_flattened().len();
        prev.pre = &data.as_flattened()[..offset_start];
        prev.post = &data.as_flattened()[offset_end..];
        data = section;

        prev = ret.push_mut(SsitSection {
            pre: section.as_flattened(),
            post: &[],
        });
    }

    Ok(ret)
}

fn read(data: &[u8; 4]) -> u32 {
    // `from_le_bytes(data).reverse_bits()` is the same as
    // `from_be_bytes(data.map(u8::reverse_bits))` and seems to optimise better
    u32::from_le_bytes(*data).reverse_bits()
}

// returns:
// - `Some(ssit)` when ssit section found. Notably, this _must_ be a subset of
//   the original data. By construction, everything before and after the subset
//   is part of the "outer" section.
// - `None` otherwise
fn find_ssit(original: *const u8, data: &[[u8; 4]]) -> Result<Option<&[[u8; 4]]>> {
    const S7_ENC: u32 = Type1::new(OpCode::Write, Addr::Rdri, Words32(1)).to_raw();
    const US_ENC: u32 = Type1::new(OpCode::Write, Addr::Cbc, Words32(4)).to_raw();

    let mut it = data.iter();
    it.find(|x| read(x) == Type1::SYNC);
    while let Some(data) = it.next() {
        let word = read(data);

        if word == Type1::SYNC {
            continue;
        }
        if word == S7_ENC {
            s7_encryption(&mut it)?;
            continue;
        }
        if word == US_ENC {
            us_encryption(&mut it)?;
            continue;
        }

        let kind = word >> 29;
        match kind {
            0 => {
                let word_count = word & 0x3ff;
                if word_count != 0 {
                    it.nth((word_count - 1) as _);
                }
                continue;
            }
            1 => {
                if let Some(x) = type1(&mut it, word)? {
                    return Ok(Some(x));
                }
            }
            2 => {
                let word_count = word & 0x03ff_ffff;
                if word_count != 0 {
                    it.nth((word_count - 1) as _);
                }
            }
            _ => {
                let offset = data.as_ptr().addr() - original.addr();
                return Err(eyre::eyre!(
                    "unknown kind at offset 0x{offset:08X}: {word:08X}"
                ));
            }
        }
    }

    Ok(None)
}

fn type1<'d>(it: &mut std::slice::Iter<'d, [u8; 4]>, word: u32) -> Result<Option<&'d [[u8; 4]]>> {
    let addr = word >> 13 & 0x3fff;
    let word_count = word & 0x3ff;

    fn consume(data: &[[u8; 4]], count: u32) -> Result<&[[u8; 4]]> {
        let count = count as usize;
        let len = data.len();
        if count > len {
            return Err(eyre::eyre!("bad word count for ssit: {count} > {len}"));
        }
        Ok(&data[..count])
    }

    const SSIT: u32 = Addr::Ssit as _;
    match (addr, word_count) {
        (SSIT, 0) => {
            let data = it.next().ok_or_eyre("eof after type1 ssit")?;
            let word = read(data);
            let kind = word >> 29;
            if kind != 2 {
                return Err(eyre::eyre!("type1 ssit not followed by type2"));
            }
            let word_count = word & 0x03ff_ffff;
            return consume(it.as_slice(), word_count).map(Some);
        }
        (SSIT, _) => {
            return consume(it.as_slice(), word_count).map(Some);
        }
        _ => (),
    }
    if word_count != 0 {
        it.nth((word_count - 1) as _);
    }
    Ok(None)
}

fn s7_encryption<'d>(it: &mut impl Iterator<Item = &'d [u8; 4]>) -> Result<()> {
    let enc_word_count = it
        .next()
        .ok_or_eyre("eof while s7 encryption special case")?;
    let enc_word_count = read(enc_word_count);
    if enc_word_count == 0 {
        return Ok(());
    }
    it.nth((enc_word_count - 1) as usize);
    Ok(())
}

fn us_encryption<'d>(it: &mut impl Iterator<Item = &'d [u8; 4]>) -> Result<()> {
    let enc_word_count = it
        .nth(3)
        .ok_or_eyre("eof while us/up encryption special case")?;
    let enc_word_count = read(enc_word_count);
    if enc_word_count == 0 {
        return Ok(());
    }
    it.skip_while(|x| read(x) == Type1::NOOP)
        .nth((enc_word_count - 1) as usize);
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::ops::Index;

    use super::*;
    use crate::_32bit::{bitstream_to_wire_order, registers::type2};

    fn idx<I1, I2>(data: &[[u8; 4]], pre: I1, post: I2) -> SsitSection<'_>
    where
        [[u8; 4]]: Index<I1, Output = [[u8; 4]]>,
        [[u8; 4]]: Index<I2, Output = [[u8; 4]]>,
    {
        SsitSection {
            pre: data[pre].as_flattened(),
            post: data[post].as_flattened(),
        }
    }

    #[test]
    fn test_parse_no_ssit() -> Result<()> {
        let data = [
            Type1::SYNC,
            Type1::NOOP,
            Type1::new(OpCode::Write, Addr::Cmd, Words32(1)).to_raw(),
            0x07,
        ];
        let data = bitstream_to_wire_order(data);
        let bitstream = parse(data.as_flattened())?;
        let expected = &[idx(&data, .., 0..0)];
        assert_eq!(bitstream, expected);
        Ok(())
    }

    #[test]
    fn test_parse_2_ssit() -> Result<()> {
        let data = [
            Type1::SYNC,
            Type1::NOOP,
            Type1::new(OpCode::Write, Addr::Cmd, Words32(1)).to_raw(),
            0x07,
            Type1::SYNC,
            Type1::new(OpCode::Write, Addr::Ssit, Words32(0)).to_raw(),
            type2(OpCode::Write, 4),
            0xffffffff,
            Type1::SYNC,
            Type1::new(OpCode::Write, Addr::Cmd, Words32(1)).to_raw(),
            0x08,
            Type1::new(OpCode::Write, Addr::Cmd, Words32(1)).to_raw(),
            0x08,
        ];
        let data = bitstream_to_wire_order(data);
        let bitstream = parse(data.as_flattened())?;

        let expected = &[idx(&data, ..7, 11..13), idx(&data, 7..11, 0..0)];
        assert_eq!(bitstream, expected);

        Ok(())
    }

    #[test]
    fn test_parse_3_ssit() -> Result<()> {
        let cmd = Type1::new(OpCode::Write, Addr::Cmd, Words32(1)).to_raw();
        let data = [
            Type1::SYNC,
            Type1::NOOP,
            cmd,
            0x07,
            Type1::SYNC,
            Type1::new(OpCode::Write, Addr::Ssit, Words32(0)).to_raw(),
            type2(OpCode::Write, 12),
            0xffffffff,
            Type1::SYNC,
            cmd,
            0x08,
            Type1::SYNC,
            Type1::new(OpCode::Write, Addr::Ssit, Words32(4)).to_raw(),
            0xffffffff,
            Type1::SYNC,
            cmd,
            0x08,
            cmd,
            0x08,
            cmd,
            0x08,
        ];
        let data = bitstream_to_wire_order(data);
        let bitstream = parse(data.as_flattened())?;

        let expected =
            &[idx(&data, ..7, 19..), idx(&data, 7..13, 17..19), idx(&data, 13..17, 0..0)];
        assert_eq!(bitstream, expected);

        Ok(())
    }
}
