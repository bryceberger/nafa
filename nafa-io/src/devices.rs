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

    #[rustfmt::skip]
    static DEVICES: &[(IdCode, DeviceInfo)] = &[
        info(0x3620093,  6, "xc7s15",     Some(134711)),
        info(0x3622093,  6, "xc7s6",      Some(134711)),
        info(0x362c093,  6, "xc7a50t",    Some(548003)),
        info(0x362d093,  6, "xc7a35t",    Some(548003)),
        info(0x362e093,  6, "xc7a15t",    Some(548003)),
        info(0x362f093,  6, "xc7s50",     Some(548003)),
        info(0x3631093,  6, "xc7a100t",   Some(956447)),
        info(0x3632093,  6, "xc7a75t",    Some(956447)),
        info(0x3636093,  6, "xc7a200t",   Some(2432663)),
        info(0x3647093,  6, "xc7k70t",    Some(752831)),
        info(0x364c093,  6, "xc7k160t",   Some(1673143)),
        info(0x3651093,  6, "xc7k325t",   Some(2860903)),
        info(0x3656093,  6, "xc7k410t",   Some(3969479)),
        info(0x3667093,  6, "xc7vx330t",  Some(3476195)),
        info(0x3671093,  6, "xc7v585t",   Some(5043715)),
        info(0x3682093,  6, "xc7vx415t",  Some(4310455)),
        info(0x3687093,  6, "xc7vx485t",  Some(5068359)),
        info(0x3691093,  6, "xc7vx690t",  Some(7183703)),
        info(0x3692093,  6, "xc7vx550t",  Some(7183703)),
        info(0x3696093,  6, "xc7vx980t",  Some(8828791)),
        info(0x36b3093, 24, "xc7v2000t",  None),
        info(0x36d5093, 24, "xc7vx1140t", None),
        info(0x3722093,  6, "xc7z010",    Some(520935)),
        info(0x3727093,  6, "xc7z020",    Some(1011391)),
        info(0x372c093,  6, "xc7z030",    Some(1494979)),
        info(0x3731093,  6, "xc7z045",    Some(3330351)),
        info(0x3736093,  6, "xc7z100",    Some(4354087)),
        info(0x3747093,  6, "xc7k355t",   Some(3512959)),
        info(0x3751093,  6, "xc7k480t",   Some(4683751)),
        info(0x3752093,  6, "xc7k420t",   Some(4683751)),
        info(0x37c2093,  6, "xc7a25t",    Some(310451)),
        info(0x37c3093,  6, "xc7a12t",    Some(310451)),
        info(0x37c4093,  6, "xc7s25",     Some(310451)),
        info(0x37c7093,  6, "xc7s100",    Some(921703)),
        info(0x37c8093,  6, "xc7s75",     Some(921703)),
        info(0x380f093, 12, "xcku085",    None),
        info(0x3822093,  6, "xcku040",    Some(4001190)),
        info(0x3823093,  6, "xcku035",    Some(4001190)),
        info(0x3824093,  6, "xcku025",    Some(4001190)),
        info(0x3842093,  6, "xcvu095",    Some(8960304)),
        info(0x3843093,  6, "xcvu080",    Some(8960304)),
        info(0x3844093,  6, "xcku095",    Some(8960304)),
        info(0x390d093, 12, "xcku115",    None),
        info(0x3919093,  6, "xcku060",    Some(6030690)),
        info(0x392d093, 12, "xcvu125",    None),
        info(0x3931093, 18, "xcvu190",    None),
        info(0x3933093, 18, "xcvu160",    None),
        info(0x3939093,  6, "xcvu065",    Some(6271770)),
        info(0x396d093, 18, "xcvu440",    None),
        info(0x484a093,  6, "xcku9p",     Some(6627180)),
        info(0x4a4e093,  6, "xcku11p",    Some(5894712)),
        info(0x4a52093,  6, "xcku13p",    Some(7174671)),
        info(0x4a56093,  6, "xcku15p",    Some(9085263)),
        info(0x4a62093,  6, "xcku5p",     Some(3857268)),
        info(0x4a63093,  6, "xcku3p",     Some(3857268)),
        info(0x4a64093,  6, "xcau25p",    Some(3857268)),
        info(0x4a65093,  6, "xcau20p",    Some(3857268)),
        info(0x4ac2093,  6, "xcau15p",    Some(1336968)),
        info(0x4ac4093,  6, "xcau10p",    Some(1336968)),
        info(0x4ace093,  6, "xcvu23p",    Some(16310712)),
        info(0x4acf093,  6, "xcku19p",    Some(16310712)),
        info(0x4af6093,  6, "xcau7p",     Some(767808)),
        info(0x4b29093, 12, "xcvu7p",     None),
        info(0x4b2b093, 12, "xcvu5p",     None),
        info(0x4b31093, 18, "xcvu9p",     None),
        info(0x4b39093,  6, "xcvu3p",     Some(6679260)),
        info(0x4b41093, 24, "xcvu29p",    None),
        info(0x4b43093, 24, "xcvu27p",    None),
        info(0x4b49093, 18, "xcvu11p",    None),
        info(0x4b51093, 24, "xcvu13p",    None),
        info(0x4b61093, 18, "xcvu57p",    None),
        info(0x4b69093,  6, "xcvu33p",    Some(7081764)),
        info(0x4b6b093,  6, "xcvu31p",    Some(7081764)),
        info(0x4b71093, 12, "xcvu35p",    None),
        info(0x4b73093, 12, "xcvu45p",    None),
        info(0x4b79093, 18, "xcvu37p",    None),
        info(0x4b7b093, 18, "xcvu47p",    None),
        info(0x4ba1093, 24, "xcvu19p",    None),
        info(0x4e80093,  6, "su35p",      Some(10606464)),
        info(0x4e81093,  6, "su10p",      Some(10606464)),
        info(0x4e82093,  6, "su25p",      Some(10606464)),
        info(0x4e88093,  6, "su50p",      Some(15728640)),
        info(0x4e90093,  6, "su55p",      Some(15728640)),
        info(0x4e98093,  6, "su100p",     Some(29360128)),
        info(0x4e99093,  6, "su65p",      None),
        info(0x4ea0093,  6, "su200p",     Some(57817728)),
        info(0x4ea1093,  6, "su150p",     Some(57817728)),
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
