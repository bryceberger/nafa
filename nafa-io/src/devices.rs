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
        irlen: u8,
        name: &'static str,
        family: Xilinx32Family,
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

    static DEVICES: &[(IdCode, DeviceInfo)] = &[
        // Kintex US
        info(0x03824093, 6, "XCKU025", F::US, Some(4001190)),
        info(0x03823093, 6, "XCKU035", F::US, Some(4001190)),
        info(0x03822093, 6, "XCKU040", F::US, Some(4001190)),
        info(0x03919093, 6, "XCKU060", F::US, Some(6030690)),
        info(0x03844093, 6, "XCKU095", F::US, Some(8960304)),
        info(0x0380F093, 12, "XCKU085", F::US, None),
        info(0x0390D093, 12, "XCKU115", F::US, None),
        // Virtex US
        info(0x03939093, 6, "XCVU065", F::US, Some(6271770)),
        info(0x03843093, 6, "XCVU080", F::US, Some(8960304)),
        info(0x03842093, 6, "XCVU095", F::US, Some(8960304)),
        info(0x0392D093, 12, "XCVU125", F::US, None),
        info(0x03933093, 18, "XCVU160", F::US, None),
        info(0x03931093, 18, "XCVU190", F::US, None),
        info(0x0396D093, 18, "XCVU440", F::US, None),
        // Spartan US+
        info(0x04E81093, 6, "SU10P", F::UP, Some(10606464)),
        info(0x04E82093, 6, "SU25P", F::UP, Some(10606464)),
        info(0x04E80093, 6, "SU35P", F::UP, Some(10606464)),
        info(0x04E88093, 6, "SU50P", F::UP, Some(15728640)),
        info(0x04E90093, 6, "SU55P", F::UP, Some(15728640)),
        info(0x04E99093, 6, "SU65P", F::UP, None),
        info(0x04E98093, 6, "SU100P", F::UP, Some(29360128)),
        info(0x04EA1093, 6, "SU150P", F::UP, Some(57817728)),
        info(0x04EA0093, 6, "SU200P", F::UP, Some(57817728)),
        // Artix US+
        info(0x04AF6093, 6, "XCAU7P", F::UP, Some(767808)),
        info(0x04AC4093, 6, "XCAU10P", F::UP, Some(1336968)),
        info(0x04AC2093, 6, "XCAU15P", F::UP, Some(1336968)),
        info(0x04A65093, 6, "XCAU20P", F::UP, Some(3857268)),
        info(0x04A64093, 6, "XCAU25P", F::UP, Some(3857268)),
        // Kintex US+
        info(0x04A63093, 6, "XCKU3P", F::UP, Some(3857268)),
        info(0x04A62093, 6, "XCKU5P", F::UP, Some(3857268)),
        info(0x0484A093, 6, "XCKU9P", F::UP, Some(6627180)),
        info(0x04A4E093, 6, "XCKU11P", F::UP, Some(5894712)),
        info(0x04A52093, 6, "XCKU13P", F::UP, Some(7174671)),
        info(0x04A56093, 6, "XCKU15P", F::UP, Some(9085263)),
        info(0x04ACF093, 6, "XCKU19P", F::UP, Some(16310712)),
        // Virtex US+
        info(0x04B39093, 6, "XCVU3P", F::UP, Some(6679260)),
        info(0x04ACE093, 6, "XCVU23P", F::UP, Some(16310712)),
        info(0x04B6B093, 6, "XCVU31P", F::UP, Some(7081764)),
        info(0x04B69093, 6, "XCVU33P", F::UP, Some(7081764)),
        // Virtex US+
        info(0x04B2B093, 12, "XCVU5P", F::UP, None),
        info(0x04B29093, 12, "XCVU7P", F::UP, None),
        info(0x04B71093, 12, "XCVU35P", F::UP, None),
        info(0x04B73093, 12, "XCVU45P", F::UP, None),
        // Virtex US+
        info(0x14B31093, 18, "XCVU9P", F::UP, None),
        info(0x14B49093, 18, "XCVU11P", F::UP, None),
        info(0x04B79093, 18, "XCVU37P", F::UP, None),
        info(0x04B7B093, 18, "XCVU47P", F::UP, None),
        info(0x04B61093, 18, "XCVU57P", F::UP, None),
        // Virtex US+
        info(0x04B51093, 24, "XCVU13P", F::UP, None),
        info(0x04BA1093, 24, "XCVU19P", F::UP, None),
        info(0x04B43093, 24, "XCVU27P", F::UP, None),
        info(0x04B41093, 24, "XCVU29P", F::UP, None),
        // Zynq US+
        info(0x04688093, 12, "XCZU1EG", F::ZP, None),
        info(0x14711093, 12, "XCZU2EG", F::ZP, None),
        info(0x14710093, 12, "XCZU3EG", F::ZP, None),
        info(0x04721093, 12, "XCZU4EG", F::ZP, None),
        info(0x04720093, 12, "XCZU5EG", F::ZP, None),
        info(0x24739093, 12, "XCZU6EG", F::ZP, None),
        info(0x14730093, 12, "XCZU7EG", F::ZP, None),
        info(0x24738093, 12, "XCZU9EG", F::ZP, None),
        info(0x04740093, 12, "XCZU11EG", F::ZP, None),
        info(0x14750093, 12, "XCZU15EG", F::ZP, None),
        info(0x14759093, 12, "XCZU17EG", F::ZP, None),
        info(0x14758093, 12, "XCZU19EG", F::ZP, None),
        info(0x147E1093, 12, "XCZU21DR", F::ZP, None),
        info(0x147E5093, 12, "XCZU25DR", F::ZP, None),
        info(0x147E4093, 12, "XCZU27DR", F::ZP, None),
        info(0x147E0093, 12, "XCZU28DR", F::ZP, None),
        info(0x147E2093, 12, "XCZU29DR", F::ZP, None),
        info(0x147E6093, 12, "XCZU39DR", F::ZP, None),
        info(0x046D4093, 12, "XCZU42DR", F::ZP, None),
        info(0x147FD093, 12, "XCZU43DR", F::ZP, None),
        info(0x147F8093, 12, "XCZU46DR", F::ZP, None),
        info(0x147FF093, 12, "XCZU47DR", F::ZP, None),
        info(0x147FB093, 12, "XCZU48DR", F::ZP, None),
        info(0x147FE093, 12, "XCZU49DR", F::ZP, None),
        info(0x046D5093, 12, "XCZU63DR", F::ZP, None),
        info(0x046D6093, 12, "XCZU64DR", F::ZP, None),
        info(0x046D1093, 12, "XCZU65DR", F::ZP, None),
        info(0x046D0093, 12, "XCZU67DR", F::ZP, None),
        info(0x04718093, 12, "XCZU3TEG", F::ZP, None),
        // Spartan-7
        info(0x03622093, 6, "XC7S6", F::S7, Some(134711)),
        info(0x03620093, 6, "XC7S15", F::S7, Some(134711)),
        info(0x037C4093, 6, "XC7S25", F::S7, Some(310451)),
        info(0x0362F093, 6, "XC7S50", F::S7, Some(548003)),
        info(0x037C8093, 6, "XC7S75", F::S7, Some(921703)),
        info(0x037C7093, 6, "XC7S100", F::S7, Some(921703)),
        // Artix-7
        info(0x037C3093, 6, "XC7A12T", F::S7, Some(310451)),
        info(0x0362E093, 6, "XC7A15T", F::S7, Some(548003)),
        info(0x037C2093, 6, "XC7A25T", F::S7, Some(310451)),
        info(0x0362D093, 6, "XC7A35T", F::S7, Some(548003)),
        info(0x0362C093, 6, "XC7A50T", F::S7, Some(548003)),
        info(0x03632093, 6, "XC7A75T", F::S7, Some(956447)),
        info(0x03631093, 6, "XC7A100T", F::S7, Some(956447)),
        info(0x03636093, 6, "XC7A200T", F::S7, Some(2432663)),
        // Kintex-7
        info(0x03647093, 6, "XC7K70T", F::S7, Some(752831)),
        info(0x0364C093, 6, "XC7K160T", F::S7, Some(1673143)),
        info(0x03651093, 6, "XC7K325T", F::S7, Some(2860903)),
        info(0x03747093, 6, "XC7K355T", F::S7, Some(3512959)),
        info(0x03656093, 6, "XC7K410T", F::S7, Some(3969479)),
        info(0x03752093, 6, "XC7K420T", F::S7, Some(4683751)),
        info(0x03751093, 6, "XC7K480T", F::S7, Some(4683751)),
        // Virtex-7
        info(0x03671093, 6, "XC7V585T", F::S7, Some(5043715)),
        info(0x036B3093, 24, "XC7V2000T", F::S7, None),
        info(0x03667093, 6, "XC7VX330T", F::S7, Some(3476195)),
        info(0x03682093, 6, "XC7VX415T", F::S7, Some(4310455)),
        info(0x03687093, 6, "XC7VX485T", F::S7, Some(5068359)),
        info(0x03692093, 6, "XC7VX550T", F::S7, Some(7183703)),
        info(0x03691093, 6, "XC7VX690T", F::S7, Some(7183703)),
        info(0x03696093, 6, "XC7VX980T", F::S7, Some(8828791)),
        info(0x036D5093, 24, "XC7VX1140T", F::S7, None),
        // Zynq-7
        info(0x03722093, 6, "XC7Z010", F::Z7, Some(520935)),
        info(0x03727093, 6, "XC7Z020", F::Z7, Some(1011391)),
        info(0x0372C093, 6, "XC7Z030", F::Z7, Some(1494979)),
        info(0x03731093, 6, "XC7Z045", F::Z7, Some(3330351)),
        info(0x03736093, 6, "XC7Z100", F::Z7, Some(4354087)),
        // ARM DAP on Zynq
        unknown(0x0BA00477, 4, "ARM_DAP"),
    ];

    DEVICES.iter().cloned()
}
