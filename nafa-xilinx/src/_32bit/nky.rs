//! <https://docs.amd.com/r/en-US/xapp1267-encryp-efuse-program/Creating-an-Encryption-Key-and-Encrypted-Bitstream>
//! > The NKY file generation occurs at the same time as bitstream generation.
//! > This file takes the same top-level name as the bit file and is placed in
//! > the
//! > same implementation directory as the encrypted bitstream. The NKY file
//! > format for monolithic devices is:
//! >
//! > ```text
//! > Device <type>;
//! > EncryptKeySelect <type>;
//! > StartIvObfuscate 00000000000000000000000000000000;
//! > Key0 0000000000000000000000000000000000000000000000000000000000000000;
//! > StartIV0 00000000000000000000000000000000;
//! > ```
//! >
//! > For 3D IC devices, the NKY file format Key and StartIV statements are
//! > extended with a second value indicating the targeted SLR index.
//! >
//! > ```text
//! > Device <type>;
//! > EncryptKeySelect <type>;
//! > Key0 0000000000000000000000000000000000000000000000000000000000000000, 0;
//! > StartIV0 00000000000000000000000000000000, 0;
//! > Key0 0000000000000000000000000000000000000000000000000000000000000000, 1;
//! > StartIV0 00000000000000000000000000000000, 1;
//! > ```
//!
//! Because of the parser implementation, the SLR number must start at 0, and
//! increase by 1 each key/IV.
//!
//! Keys/IVs may be larger than required, but not smaller. If larger, the
//! leftmost hex digits are used.

use std::cell::Cell;

use nom::{
    IResult, Parser,
    branch::alt,
    bytes::complete::{tag, take_while_m_n},
    character::complete::{digit1, line_ending, not_line_ending, space0, space1},
    combinator::{all_consuming, complete, cut, opt, verify},
    error::{FromExternalError, ParseError},
    multi::fold_many0,
    sequence::{delimited, preceded},
};

pub struct Nky {
    pub keys: Vec<[u8; 32]>,
    pub ivs: Vec<[u8; 12]>,
}

#[derive(Debug, thiserror::Error)]
#[error("line number {line_no}: {kind:?}")]
pub struct NkyParseError {
    pub line_no: usize,
    pub kind: nom::error::ErrorKind,
}

impl Nky {
    pub fn parse(content: &str) -> Result<Nky, NkyParseError> {
        parse_lines(content).map(|(_, n)| n).map_err(|e| {
            let (byte_offset, kind) = match &e {
                nom::Err::Incomplete(_) => unreachable!(),
                nom::Err::Error(e) | nom::Err::Failure(e) => (
                    e.input.as_ptr() as usize - content.as_ptr() as usize,
                    e.code,
                ),
            };
            let line_no = content.as_bytes()[..byte_offset]
                .iter()
                .filter(|x| **x == b'\n')
                .count();
            NkyParseError { line_no, kind }
        })
    }
}

fn parse_line<'i, const N: usize, O, E>(
    start: impl Parser<&'i str, Output = O, Error = E>,
) -> impl Parser<&'i str, Output = ([u8; N], Option<usize>), Error = E>
where
    E: ParseError<&'i str>,
    E: FromExternalError<&'i str, std::num::ParseIntError>,
    E: FromExternalError<&'i str, hex::FromHexError>,
{
    let parse_hex =
        take_while_m_n(N * 2, usize::MAX, |c: char| c.is_ascii_hexdigit()).map_res(|x: &str| {
            let mut ret = [0u8; N];
            hex::decode_to_slice(&x[..N * 2], &mut ret)?;
            Ok::<_, hex::FromHexError>(ret)
        });
    let parse_num = digit1.map_res(|x: &str| x.parse::<usize>());
    let parse_slr = preceded((tag(","), space0), parse_num);
    delimited(
        (start, space1),
        (cut(parse_hex), opt(parse_slr)),
        (tag(";"), line_ending),
    )
}

fn parse_lines(input: &str) -> IResult<&str, Nky> {
    enum ParseBranch {
        Key([u8; 32]),
        Iv([u8; 12]),
        Ignore,
    }

    let key = Cell::new(0);
    let iv = Cell::new(0);
    let incr_key = move || {
        let k = key.get();
        key.set(k + 1);
        k
    };
    let incr_iv = move || {
        let i = iv.get();
        iv.set(i + 1);
        i
    };

    let start_keys = alt((tag("Key0"), tag("Key 0")));
    let parse_key = verify(parse_line(start_keys), |n| n.1.unwrap_or(0) == incr_key());

    let start_ivs = parse_line(alt((tag("StartIV0"), tag("Key StartCBC"))));
    let parse_iv = verify(start_ivs, |n| n.1.unwrap_or(0) == incr_iv());

    let parser = alt((
        parse_key.map(|(key, _)| ParseBranch::Key(key)),
        parse_iv.map(|(iv, _)| ParseBranch::Iv(iv)),
        (not_line_ending, line_ending).map(|_| ParseBranch::Ignore),
    ));

    let full_parser = fold_many0(
        parser,
        || (Vec::new(), Vec::new()),
        |(mut keys, mut ivs), branch| {
            match branch {
                ParseBranch::Key(key) => keys.push(key),
                ParseBranch::Iv(iv) => ivs.push(iv),
                ParseBranch::Ignore => (),
            }
            (keys, ivs)
        },
    )
    .map(|(keys, ivs)| Nky { keys, ivs });

    all_consuming(complete(full_parser)).parse(input)
}

#[cfg(test)]
mod tests {
    use eyre::Result;

    use super::*;

    #[test]
    fn test_s7_nky() -> Result<()> {
        let nky = Nky::parse(
            "\
Device xc7a35t;
Key 0 0000000000000000000000000000000000000000000000000000000000000000;
Key StartCBC 00000000000000000000000000000000;
Key HMAC 0000000000000000000000000000000000000000000000000000000000000000;
",
        )?;
        assert_eq!(nky.keys, &[[0x00; 32]]);
        assert_eq!(nky.ivs, &[[0x00; 12]]);
        Ok(())
    }

    #[test]
    fn test_us_nky() -> Result<()> {
        let nky = Nky::parse(
            "\
Device xcku5p;
EncryptKeySelect BBRAM;
Key0 ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff;
StartIV0 bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb;
",
        )?;
        assert_eq!(nky.keys, &[[0xff; 32]]);
        assert_eq!(nky.ivs, &[[0xbb; 12]]);
        Ok(())
    }

    #[test]
    fn test_us_slr_nky() -> Result<()> {
        let nky = Nky::parse(
            "\
Device xcku5p;
EncryptKeySelect BBRAM;
Key0 aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa, 0;
StartIV0 11111111111111111111111111111111, 0;
Key0 cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc, 1;
# note that the IV is longer than required, so leftmost digits are used
StartIV0 333333333333333333333333aabbccdd, 1;
",
        )?;
        assert_eq!(nky.keys, &[[0xaa; 32], [0xcc; 32]]);
        assert_eq!(nky.ivs, &[[0x11; 12], [0x33; 12]]);
        Ok(())
    }
}
