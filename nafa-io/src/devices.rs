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
    /// Zync 7000
    Z7,
    /// Zync Ultrascale+
    ZP,
}

/// Returns iterator of `(idcode, info)`. Intended to be collected into a
/// `HashMap`, to be passed to [`crate::Controller::new`].
pub fn builtin() -> impl Iterator<Item = (IdCode, DeviceInfo)> {
    [].into_iter().chain(xilinx()).chain(intel())
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

    const fn info(
        idcode: u32,
        family: Xilinx32Family,
        irlen: u8,
        name: &'static str,
        readback: Option<usize>,
    ) -> (IdCode, DeviceInfo) {
        let specific = Specific::Xilinx32(Xilinx32Info {
            slr: irlen / 6,
            family,
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

    const fn unknown(idcode: u32, irlen: u8, name: &'static str) -> (IdCode, DeviceInfo) {
        let info = DeviceInfo {
            irlen: Bits(irlen),
            name,
            specific: Specific::Unknown,
        };
        (id(idcode), info)
    }

    #[rustfmt::skip]
    static DEVICES: &[(IdCode, DeviceInfo)] = &[
        info(0x3620093, F::S7,  6, "xc7s15",     Some(134711)),
        info(0x3622093, F::S7,  6, "xc7s6",      Some(134711)),
        info(0x362c093, F::S7,  6, "xc7a50t",    Some(548003)),
        info(0x362d093, F::S7,  6, "xc7a35t",    Some(548003)),
        info(0x362e093, F::S7,  6, "xc7a15t",    Some(548003)),
        info(0x362f093, F::S7,  6, "xc7s50",     Some(548003)),
        info(0x3631093, F::S7,  6, "xc7a100t",   Some(956447)),
        info(0x3632093, F::S7,  6, "xc7a75t",    Some(956447)),
        info(0x3636093, F::S7,  6, "xc7a200t",   Some(2432663)),
        info(0x3647093, F::S7,  6, "xc7k70t",    Some(752831)),
        info(0x364c093, F::S7,  6, "xc7k160t",   Some(1673143)),
        info(0x3651093, F::S7,  6, "xc7k325t",   Some(2860903)),
        info(0x3656093, F::S7,  6, "xc7k410t",   Some(3969479)),
        info(0x3667093, F::S7,  6, "xc7vx330t",  Some(3476195)),
        info(0x3671093, F::S7,  6, "xc7v585t",   Some(5043715)),
        info(0x3682093, F::S7,  6, "xc7vx415t",  Some(4310455)),
        info(0x3687093, F::S7,  6, "xc7vx485t",  Some(5068359)),
        info(0x3691093, F::S7,  6, "xc7vx690t",  Some(7183703)),
        info(0x3692093, F::S7,  6, "xc7vx550t",  Some(7183703)),
        info(0x3696093, F::S7,  6, "xc7vx980t",  Some(8828791)),
        info(0x36b3093, F::S7, 24, "xc7v2000t",  None),
        info(0x36d5093, F::S7, 24, "xc7vx1140t", None),
        info(0x3722093, F::Z7,  6, "xc7z010",    Some(520935)),
        info(0x3727093, F::Z7,  6, "xc7z020",    Some(1011391)),
        info(0x372c093, F::Z7,  6, "xc7z030",    Some(1494979)),
        info(0x3731093, F::Z7,  6, "xc7z045",    Some(3330351)),
        info(0x3736093, F::Z7,  6, "xc7z100",    Some(4354087)),
        info(0x3747093, F::S7,  6, "xc7k355t",   Some(3512959)),
        info(0x3751093, F::S7,  6, "xc7k480t",   Some(4683751)),
        info(0x3752093, F::S7,  6, "xc7k420t",   Some(4683751)),
        info(0x37c2093, F::S7,  6, "xc7a25t",    Some(310451)),
        info(0x37c3093, F::S7,  6, "xc7a12t",    Some(310451)),
        info(0x37c4093, F::S7,  6, "xc7s25",     Some(310451)),
        info(0x37c7093, F::S7,  6, "xc7s100",    Some(921703)),
        info(0x37c8093, F::S7,  6, "xc7s75",     Some(921703)),
        info(0x380f093, F::US, 12, "xcku085",    None),
        info(0x3822093, F::US,  6, "xcku040",    Some(4001190)),
        info(0x3823093, F::US,  6, "xcku035",    Some(4001190)),
        info(0x3824093, F::US,  6, "xcku025",    Some(4001190)),
        info(0x3842093, F::US,  6, "xcvu095",    Some(8960304)),
        info(0x3843093, F::US,  6, "xcvu080",    Some(8960304)),
        info(0x3844093, F::US,  6, "xcku095",    Some(8960304)),
        info(0x390d093, F::US, 12, "xcku115",    None),
        info(0x3919093, F::US,  6, "xcku060",    Some(6030690)),
        info(0x392d093, F::US, 12, "xcvu125",    None),
        info(0x3931093, F::US, 18, "xcvu190",    None),
        info(0x3933093, F::US, 18, "xcvu160",    None),
        info(0x3939093, F::US,  6, "xcvu065",    Some(6271770)),
        info(0x396d093, F::US, 18, "xcvu440",    None),
        info(0x4688093, F::ZP, 12, "xczu1eg",    None),
        info(0x46d0093, F::ZP, 12, "xczu67dr",   None),
        info(0x46d1093, F::ZP, 12, "xczu65dr",   None),
        info(0x46d4093, F::ZP, 12, "xczu42dr",   None),
        info(0x46d5093, F::ZP, 12, "xczu63dr",   None),
        info(0x46d6093, F::ZP, 12, "xczu64dr",   None),
        info(0x4710093, F::ZP, 12, "xczu3eg",    None),
        info(0x4711093, F::ZP, 12, "xczu2eg",    None),
        info(0x4718093, F::ZP, 12, "xczu3teg",   None),
        info(0x4720093, F::ZP, 12, "xczu5eg",    None),
        info(0x4721093, F::ZP, 12, "xczu4eg",    None),
        info(0x4730093, F::ZP, 12, "xczu7eg",    None),
        info(0x4738093, F::ZP, 12, "xczu9eg",    None),
        info(0x4739093, F::ZP, 12, "xczu6eg",    None),
        info(0x4740093, F::ZP, 12, "xczu11eg",   None),
        info(0x4750093, F::ZP, 12, "xczu15eg",   None),
        info(0x4758093, F::ZP, 12, "xczu19eg",   None),
        info(0x4759093, F::ZP, 12, "xczu17eg",   None),
        info(0x47e0093, F::ZP, 12, "xczu28dr",   None),
        info(0x47e1093, F::ZP, 12, "xczu21dr",   None),
        info(0x47e2093, F::ZP, 12, "xczu29dr",   None),
        info(0x47e4093, F::ZP, 12, "xczu27dr",   None),
        info(0x47e5093, F::ZP, 12, "xczu25dr",   None),
        info(0x47e6093, F::ZP, 12, "xczu39dr",   None),
        info(0x47f8093, F::ZP, 12, "xczu46dr",   None),
        info(0x47fb093, F::ZP, 12, "xczu48dr",   None),
        info(0x47fd093, F::ZP, 12, "xczu43dr",   None),
        info(0x47fe093, F::ZP, 12, "xczu49dr",   None),
        info(0x47ff093, F::ZP, 12, "xczu47dr",   None),
        info(0x484a093, F::UP,  6, "xcku9p",     Some(6627180)),
        info(0x4a4e093, F::UP,  6, "xcku11p",    Some(5894712)),
        info(0x4a52093, F::UP,  6, "xcku13p",    Some(7174671)),
        info(0x4a56093, F::UP,  6, "xcku15p",    Some(9085263)),
        info(0x4a62093, F::UP,  6, "xcku5p",     Some(3857268)),
        info(0x4a63093, F::UP,  6, "xcku3p",     Some(3857268)),
        info(0x4a64093, F::UP,  6, "xcau25p",    Some(3857268)),
        info(0x4a65093, F::UP,  6, "xcau20p",    Some(3857268)),
        info(0x4ac2093, F::UP,  6, "xcau15p",    Some(1336968)),
        info(0x4ac4093, F::UP,  6, "xcau10p",    Some(1336968)),
        info(0x4ace093, F::UP,  6, "xcvu23p",    Some(16310712)),
        info(0x4acf093, F::UP,  6, "xcku19p",    Some(16310712)),
        info(0x4af6093, F::UP,  6, "xcau7p",     Some(767808)),
        info(0x4b29093, F::UP, 12, "xcvu7p",     None),
        info(0x4b2b093, F::UP, 12, "xcvu5p",     None),
        info(0x4b31093, F::UP, 18, "xcvu9p",     None),
        info(0x4b39093, F::UP,  6, "xcvu3p",     Some(6679260)),
        info(0x4b41093, F::UP, 24, "xcvu29p",    None),
        info(0x4b43093, F::UP, 24, "xcvu27p",    None),
        info(0x4b49093, F::UP, 18, "xcvu11p",    None),
        info(0x4b51093, F::UP, 24, "xcvu13p",    None),
        info(0x4b61093, F::UP, 18, "xcvu57p",    None),
        info(0x4b69093, F::UP,  6, "xcvu33p",    Some(7081764)),
        info(0x4b6b093, F::UP,  6, "xcvu31p",    Some(7081764)),
        info(0x4b71093, F::UP, 12, "xcvu35p",    None),
        info(0x4b73093, F::UP, 12, "xcvu45p",    None),
        info(0x4b79093, F::UP, 18, "xcvu37p",    None),
        info(0x4b7b093, F::UP, 18, "xcvu47p",    None),
        info(0x4ba1093, F::UP, 24, "xcvu19p",    None),
        info(0x4e80093, F::UP,  6, "su35p",      Some(10606464)),
        info(0x4e81093, F::UP,  6, "su10p",      Some(10606464)),
        info(0x4e82093, F::UP,  6, "su25p",      Some(10606464)),
        info(0x4e88093, F::UP,  6, "su50p",      Some(15728640)),
        info(0x4e90093, F::UP,  6, "su55p",      Some(15728640)),
        info(0x4e98093, F::UP,  6, "su100p",     Some(29360128)),
        info(0x4e99093, F::UP,  6, "su65p",      None),
        info(0x4ea0093, F::UP,  6, "su200p",     Some(57817728)),
        info(0x4ea1093, F::UP,  6, "su150p",     Some(57817728)),
        unknown(0xba00477, 4, "arm_dap"),
    ];

    DEVICES.iter().cloned()
}
