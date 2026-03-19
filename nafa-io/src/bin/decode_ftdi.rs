#![feature(gen_blocks)]
#![feature(slice_from_ptr_range)]

use std::{collections::HashMap, io::BufRead, sync::LazyLock};

use nafa_io::{
    ShortHex,
    jtag::{GRAPH, Path, State},
};

fn main() -> std::io::Result<()> {
    for line in std::io::stdin().lock().lines() {
        match line {
            Ok(line) => {
                let Some((state, data)) = line.split_once(' ') else {
                    eprintln!("could not find state");
                    continue;
                };
                let Some(state) = SJ.get(state) else {
                    eprintln!("bad state: {state}");
                    continue;
                };
                match hex::decode(data) {
                    Ok(data) => print_table(*state, decode_line(&data)),
                    Err(e) => eprintln!("while decoding hex: {e}"),
                }
            }
            Err(e) => eprintln!("while decoding utf-8: {e}"),
        }
    }
    Ok(())
}

static SJ: LazyLock<HashMap<&'static str, State>> = LazyLock::new(|| {
    HashMap::from([
        ("TestLogicReset", State::TestLogicReset),
        ("RunTestIdle", State::RunTestIdle),
        ("SelectDR", State::SelectDR),
        ("CaptureDR", State::CaptureDR),
        ("ShiftDR", State::ShiftDR),
        ("Exit1DR", State::Exit1DR),
        ("PauseDR", State::PauseDR),
        ("Exit2DR", State::Exit2DR),
        ("UpdateDR", State::UpdateDR),
        ("SelectIR", State::SelectIR),
        ("CaptureIR", State::CaptureIR),
        ("ShiftIR", State::ShiftIR),
        ("Exit1IR", State::Exit1IR),
        ("PauseIR", State::PauseIR),
        ("Exit2IR", State::Exit2IR),
        ("UpdateIR", State::UpdateIR),
    ])
});
static JS: LazyLock<HashMap<State, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        (State::TestLogicReset, "TestLogicReset"),
        (State::RunTestIdle, "RunTestIdle"),
        (State::SelectDR, "SelectDR"),
        (State::CaptureDR, "CaptureDR"),
        (State::ShiftDR, "ShiftDR"),
        (State::Exit1DR, "Exit1DR"),
        (State::PauseDR, "PauseDR"),
        (State::Exit2DR, "Exit2DR"),
        (State::UpdateDR, "UpdateDR"),
        (State::SelectIR, "SelectIR"),
        (State::CaptureIR, "CaptureIR"),
        (State::ShiftIR, "ShiftIR"),
        (State::Exit1IR, "Exit1IR"),
        (State::PauseIR, "PauseIR"),
        (State::Exit2IR, "Exit2IR"),
        (State::UpdateIR, "UpdateIR"),
    ])
});
fn print_table<'d>(mut state: State, commands: impl Iterator<Item = FtdiCommand<'d>>) {
    println!("                     | mode | r | state           | tms        | tdi");
    let tms_len = 10;
    let empty = "";
    let t = "✓";
    let f = "✗";
    for command in commands {
        let consumed = format!("{}", ShortHex(command.consumed));
        match command.inner {
            FtdiCommandInner::Tms {
                len,
                path,
                tdi,
                read,
            } => {
                println!(
                    "{consumed:<20} | tms  | {read} | {state:<15} | {path:0len$b}{empty:extra$} | \
                     {tdi}",
                    read = if read { t } else { f },
                    state = "",
                    path = (path.reverse_bits() >> (8 - len)) & ((1 << len) - 1),
                    len = usize::from(len),
                    extra = tms_len - usize::from(len),
                    tdi = if tdi { 1 } else { 0 },
                );
                let path = Path::from_clocked(path, len);
                for dir in path {
                    state = GRAPH[state][dir];
                }
            }

            FtdiCommandInner::Bits { len, data, read } => println!(
                "{consumed:<20} | bit  | {read} | {state:<15} | {empty:tms_len$} | 0b{data:0len$b}",
                read = if read { t } else { f },
                state = JS[&state],
                data = data & ((1 << len) - 1),
                len = usize::from(len),
            ),

            FtdiCommandInner::Bytes { data, read } => println!(
                "{consumed:<20} | byte | {read} | {state:<15} | {empty:tms_len$} | 0x{data}",
                read = if read { t } else { f },
                state = JS[&state],
                data = nafa_io::ShortHex(data),
            ),

            FtdiCommandInner::Read { len } => println!(
                "{consumed:<20} | read | {read} | {state:<15} | {empty:tms_len$} | ({len})",
                read = t,
                state = JS[&state],
            ),

            FtdiCommandInner::ClockBits { bits } => println!(
                "{consumed:<20} | clk  |   | {state:<15} | {empty:tms_len$} | ({bits} bits)",
                state = JS[&state],
            ),
            FtdiCommandInner::ClockBytes { bytes } => println!(
                "{consumed:<20} | clk  |   | {state:<15} | {empty:tms_len$} | ({bytes} bytes)",
                state = JS[&state],
            ),
        }
    }
}

struct FtdiCommand<'d> {
    consumed: &'d [u8],
    inner: FtdiCommandInner<'d>,
}
enum FtdiCommandInner<'d> {
    Tms {
        len: u8,
        path: u8,
        tdi: bool,
        read: bool,
    },
    Bits {
        len: u8,
        data: u8,
        read: bool,
    },
    Bytes {
        data: &'d [u8],
        read: bool,
    },
    Read {
        len: u16,
    },
    ClockBits {
        bits: u8,
    },
    ClockBytes {
        bytes: u16,
    },
}

fn decode_line(mut line: &[u8]) -> impl Iterator<Item = FtdiCommand<'_>> {
    use nafa_io::ftdi::flags;

    gen move {
        while let Some((cmd, rest)) = line.split_first() {
            let start = line.as_ptr();
            let y = |rest: &[_], inner| FtdiCommand {
                consumed: unsafe { std::slice::from_ptr_range(start..rest.as_ptr()) },
                inner,
            };
            if *cmd == 0x8f {
                let Some(([len_high, len_low], rest)) = rest.split_first_chunk() else {
                    break;
                };
                let len = u16::from(*len_low) | u16::from(*len_high) << 8;
                line = rest;
                yield y(rest, FtdiCommandInner::ClockBytes { bytes: len });
            } else if *cmd == 0x8e {
                let Some((bits, rest)) = rest.split_first() else {
                    break;
                };
                line = rest;
                yield y(rest, FtdiCommandInner::ClockBits { bits: *bits });
            } else if cmd & flags::BITMODE != 0 {
                let read = cmd & flags::DO_READ != 0;
                let Some(([len, data], rest)) = rest.split_first_chunk() else {
                    break;
                };
                let len = len + 1;
                line = rest;
                if cmd & flags::WRITE_TMS != 0 {
                    yield y(
                        rest,
                        FtdiCommandInner::Tms {
                            len,
                            path: data & 0x7f,
                            tdi: data & 0x80 != 0,
                            read,
                        },
                    );
                } else {
                    yield y(
                        rest,
                        FtdiCommandInner::Bits {
                            len,
                            data: *data,
                            read,
                        },
                    )
                }
            } else {
                let read = cmd & flags::DO_READ != 0;
                let write = cmd & flags::DO_WRITE != 0;
                let Some(([len_low, len_high], rest)) = rest.split_first_chunk() else {
                    break;
                };
                let len = (usize::from(*len_low) | usize::from(*len_high) << 8) + 1;
                if write {
                    let Some((data, rest)) = rest.split_at_checked(len) else {
                        break;
                    };
                    line = rest;
                    yield y(rest, FtdiCommandInner::Bytes { data, read });
                } else {
                    line = rest;
                    yield y(rest, FtdiCommandInner::Read { len: len as u16 });
                }
            }
        }
    }
}
