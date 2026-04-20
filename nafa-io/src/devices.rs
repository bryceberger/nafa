use facet::Facet;

use crate::{
    jtag::IdCode,
    units::{Bits, Words32},
};

#[derive(Clone, Debug)]
pub struct DeviceInfo {
    pub irlen: Bits<u8>,
    pub name: &'static str,
    pub specific: Specific,
}

#[derive(Clone, Debug)]
pub enum Specific {
    Unknown,
    Xilinx32(Xilinx32Info),
    XilinxZynq(XilinxZynqInfo),
    Intel,
}

pub trait GetSpecific<T> {
    fn get(&self) -> Option<&T>;
}
impl GetSpecific<Xilinx32Info> for Specific {
    fn get(&self) -> Option<&Xilinx32Info> {
        match self {
            Specific::Xilinx32(info) => Some(info),
            _ => None,
        }
    }
}
impl GetSpecific<XilinxZynqInfo> for Specific {
    fn get(&self) -> Option<&XilinxZynqInfo> {
        match self {
            Specific::XilinxZynq(info) => Some(info),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Xilinx32Info {
    pub family: Xilinx32Family,
    pub slr: u8,
    pub readback: Option<Words32<usize>>,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Facet)]
pub enum Xilinx32Family {
    /// 7-series
    S7,
    /// Ultrascale
    US,
    /// Ultrascale+
    UP,
}

#[derive(Clone, Debug)]
pub struct XilinxZynqInfo {}

/// Returns iterator of `(idcode, info)`. Intended to be collected into a
/// `HashMap`, to be passed to [`crate::Controller::new`].
pub fn builtin() -> impl Iterator<Item = (IdCode, DeviceInfo)> {
    [].into_iter()
        .chain(xilinx())
        .chain(xilinx_zynq())
        .chain(intel())
}

const fn id(code: u32) -> IdCode {
    IdCode::new(code)
}

fn intel() -> impl Iterator<Item = (IdCode, DeviceInfo)> {
    const fn info(idcode: u32, irlen: u8, name: &'static str) -> (IdCode, DeviceInfo) {
        let info = DeviceInfo {
            irlen: Bits(irlen),
            name,
            specific: Specific::Intel,
        };
        (id(idcode), info)
    }

    static DEVICES: &[(IdCode, DeviceInfo)] = &[
        info(0x020D10DD, 10, "vtap10"),
        info(0x020F30DD, 10, "10CL025"),
        info(0x031820DD, 10, "10M08S"),
    ];

    DEVICES.iter().cloned()
}

fn xilinx() -> impl Iterator<Item = (IdCode, DeviceInfo)> {
    use Xilinx32Family as F;

    const fn get_family(x: u32) -> F {
        match ((x >> 21) & 0x7f) as u8 {
            0x1b => F::S7,
            0x1c => F::US,
            0x24 | 0x25 | 0x27 => F::UP,
            _ => unreachable!(),
        }
    }

    const fn info(
        idcode: u32,
        irlen: u8,
        name: &'static str,
        readback: Option<usize>,
    ) -> (IdCode, DeviceInfo) {
        let specific = Specific::Xilinx32(Xilinx32Info {
            slr: irlen / 6,
            family: get_family(idcode),
            readback: match readback {
                Some(r) => Some(Words32(r)),
                None => None,
            },
        });
        let info = DeviceInfo {
            irlen: Bits(irlen),
            name,
            specific,
        };
        assert!(irlen.is_multiple_of(6));
        (id(idcode), info)
    }

    static DEVICES: &[(IdCode, DeviceInfo)] = &[
        info(0x3620093, 6, "xc7s15", Some(134229)),
        info(0x3622093, 6, "xc7s6", Some(134229)),
        info(0x362c093, 6, "xc7a50t", Some(547521)),
        info(0x362d093, 6, "xc7a35ti", Some(547521)),
        info(0x362e093, 6, "xc7a15t", Some(547521)),
        info(0x362f093, 6, "xc7s50", Some(547521)),
        info(0x3631093, 6, "xc7a100ti", Some(955965)),
        info(0x3632093, 6, "xc7a75t", Some(955965)),
        info(0x3636093, 6, "xc7a200t", Some(2432181)),
        info(0x3647093, 6, "xc7k70t", Some(752349)),
        info(0x364c093, 6, "xc7k160t", Some(1672661)),
        info(0x3722093, 6, "xc7z010i", Some(520453)),
        info(0x3723093, 6, "xc7z007s", Some(520453)),
        info(0x3727093, 6, "xc7z020i", Some(1010909)),
        info(0x3728093, 6, "xc7z014s", Some(1010909)),
        info(0x372c093, 6, "xc7z030i", Some(1494497)),
        info(0x373b093, 6, "xc7z015i", Some(877185)),
        info(0x373c093, 6, "xc7z012s", Some(877185)),
        info(0x37c2093, 6, "xc7a25t", Some(309969)),
        info(0x37c3093, 6, "xc7a12ti", Some(309969)),
        info(0x37c4093, 6, "xc7s25", Some(309969)),
        info(0x37c7093, 6, "xc7s100", Some(921221)),
        info(0x37c8093, 6, "xc7s75", Some(921221)),
        info(0x3823093, 6, "xcku035", Some(4001323)),
        info(0x3824093, 6, "xcku025", Some(4001323)),
        info(0x4826093, 6, "xczu1cg", Some(742258)),
        info(0x4a42093, 6, "xczu3cg", Some(1391770)),
        info(0x4a43093, 6, "xczu2cg", Some(1391770)),
        info(0x4a44093, 6, "xck24", Some(1391770)),
        info(0x4a46093, 6, "xczu5cg", Some(1949026)),
        info(0x4a47093, 6, "xczu4cg", Some(1949026)),
        info(0x4a49093, 6, "xck26", Some(1949026)),
        info(0x4a5a093, 6, "xczu7cg", Some(4827376)),
        info(0x4a5c093, 6, "xcu30", Some(4827376)),
        info(0x4a62093, 6, "xcku5p", Some(3857386)),
        info(0x4a63093, 6, "xcku3p", Some(3857386)),
        info(0x4a64093, 6, "xcau25p", Some(3857386)),
        info(0x4a65093, 6, "xcau20p", Some(3857386)),
        info(0x4ac2093, 6, "xcau15p", Some(1337086)),
        info(0x4ac4093, 6, "xcau10p", Some(1337086)),
        info(0x4ad5093, 6, "xcu26", None),
        info(0x4af2093, 6, "xczu3tcg", Some(1301653)),
        info(0x4af6093, 6, "xcau7p", Some(767926)),
        info(0x4b37093, 18, "xcu200", None),
        info(0x4b71093, 12, "xcvu35p", None),
        info(0x4b77093, 12, "xcu50", None),
        info(0x4b79093, 18, "xcvu37p", None),
        info(0x4b7d093, 18, "xcu55c", None),
    ];

    DEVICES.iter().cloned()
}

fn xilinx_zynq() -> impl Iterator<Item = (IdCode, DeviceInfo)> {
    const fn info(idcode: u32, irlen: u8, name: &'static str) -> (IdCode, DeviceInfo) {
        let specific = Specific::XilinxZynq(XilinxZynqInfo {});
        let info = DeviceInfo {
            irlen: Bits(irlen),
            name,
            specific,
        };
        assert!(irlen.is_multiple_of(6));
        (id(idcode), info)
    }
    const fn unknown(idcode: u32, irlen: u8, name: &'static str) -> (IdCode, DeviceInfo) {
        let info = DeviceInfo {
            irlen: Bits(irlen),
            name,
            specific: Specific::Unknown,
        };
        (id(idcode), info)
    }

    static DEVICES: &[(IdCode, DeviceInfo)] = &[
        info(0x4688093, 12, "xczu1eg"),
        info(0x46d0093, 12, "xczu67dr"),
        info(0x46d1093, 12, "xczu65dr"),
        info(0x46d4093, 12, "xczu42dr"),
        info(0x46d5093, 12, "xczu63dr"),
        info(0x46d6093, 12, "xczu64dr"),
        info(0x4710093, 12, "xczu3eg"),
        info(0x4711093, 12, "xczu2eg"),
        info(0x4718093, 12, "xczu3teg"),
        info(0x4720093, 12, "xczu5eg"),
        info(0x4721093, 12, "xczu4eg"),
        info(0x4730093, 12, "xczu7eg"),
        info(0x4738093, 12, "xczu9eg"),
        info(0x4739093, 12, "xczu6eg"),
        info(0x4740093, 12, "xczu11eg"),
        info(0x4750093, 12, "xczu15eg"),
        info(0x4758093, 12, "xczu19eg"),
        info(0x4759093, 12, "xczu17eg"),
        info(0x47e0093, 12, "xczu28dr"),
        info(0x47e1093, 12, "xczu21dr"),
        info(0x47e2093, 12, "xczu29dr"),
        info(0x47e4093, 12, "xczu27dr"),
        info(0x47e5093, 12, "xczu25dr"),
        info(0x47e6093, 12, "xczu39dr"),
        info(0x47f8093, 12, "xczu46dr"),
        info(0x47fb093, 12, "xczu48dr"),
        info(0x47fd093, 12, "xczu43dr"),
        info(0x47fe093, 12, "xczu49dr"),
        info(0x47ff093, 12, "xczu47dr"),
        unknown(0xba00477, 4, "arm_dap"),
    ];

    DEVICES.iter().cloned()
}
