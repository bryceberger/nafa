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
    XilinxVersal(XilinxVersalInfo),
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

impl GetSpecific<XilinxVersalInfo> for Specific {
    fn get(&self) -> Option<&XilinxVersalInfo> {
        match self {
            Specific::XilinxVersal(info) => Some(info),
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

#[derive(Clone, Debug)]
pub struct XilinxVersalInfo {}

/// Returns iterator of `(idcode, info)`. Intended to be collected into a
/// `HashMap`, to be passed to [`crate::Controller::new`].
pub fn builtin() -> impl Iterator<Item = (IdCode, DeviceInfo)> {
    [].into_iter()
        .chain(xilinx())
        .chain(xilinx_zynq())
        .chain(xilinx_versal())
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
        info(0x3651093, 6, "xc7k325ti", Some(2860421)),
        info(0x3656093, 6, "xc7k410t", Some(3968997)),
        info(0x3667093, 6, "xc7vx330t", Some(3475713)),
        info(0x3671093, 6, "xc7v585t", Some(5043233)),
        info(0x3682093, 6, "xc7vx415t", Some(4309973)),
        info(0x3685093, 6, "xc7vx415t_CIV", Some(4309973)),
        info(0x3687093, 6, "xc7vx485t", Some(5067877)),
        info(0x3691093, 6, "xc7vx690t", Some(7183221)),
        info(0x3692093, 6, "xc7vx550t", Some(7183221)),
        info(0x3693093, 6, "xc7vx550t_CIV", Some(7183221)),
        info(0x3694093, 6, "xc7vx690t_CIV", Some(7183221)),
        info(0x3696093, 6, "xc7vx980t", Some(8828309)),
        info(0x36b3093, 24, "xc7v2000t", None),
        info(0x36d5093, 24, "xc7vx1140t", None),
        info(0x36d9093, 18, "xc7vh580t", None),
        info(0x36db093, 30, "xc7vh870t", None),
        info(0x3722093, 6, "xc7z010i", Some(520453)),
        info(0x3723093, 6, "xc7z007s", Some(520453)),
        info(0x3727093, 6, "xc7z020i", Some(1010909)),
        info(0x3728093, 6, "xc7z014s", Some(1010909)),
        info(0x372c093, 6, "xc7z030i", Some(1494497)),
        info(0x3731093, 6, "xc7z045", Some(3329869)),
        info(0x3732093, 6, "xc7z035", Some(3329869)),
        info(0x3736093, 6, "xc7z100i", Some(4353605)),
        info(0x373b093, 6, "xc7z015i", Some(877185)),
        info(0x373c093, 6, "xc7z012s", Some(877185)),
        info(0x3747093, 6, "xc7k355ti", Some(3512477)),
        info(0x3751093, 6, "xc7k480ti", Some(4683269)),
        info(0x3752093, 6, "xc7k420t", Some(4683269)),
        info(0x37c2093, 6, "xc7a25t", Some(309969)),
        info(0x37c3093, 6, "xc7a12ti", Some(309969)),
        info(0x37c4093, 6, "xc7s25", Some(309969)),
        info(0x37c7093, 6, "xc7s100", Some(921221)),
        info(0x37c8093, 6, "xc7s75", Some(921221)),
        info(0x380f093, 12, "xcku085", None),
        info(0x381b093, 6, "xcku060_CIV", Some(6030823)),
        info(0x3822093, 6, "xcku040", Some(4001323)),
        info(0x3823093, 6, "xcku035", Some(4001323)),
        info(0x3824093, 6, "xcku025", Some(4001323)),
        info(0x3842093, 6, "xcvu095", Some(8960437)),
        info(0x3843093, 6, "xcvu080", Some(8960437)),
        info(0x3844093, 6, "xcku095", Some(8960437)),
        info(0x3845093, 6, "xcvu080_CIV", Some(8960437)),
        info(0x390d093, 12, "xcku115", None),
        info(0x3919093, 6, "xcku060", Some(6030823)),
        info(0x392d093, 12, "xcvu125", None),
        info(0x392f093, 12, "xcvu125_CIV", None),
        info(0x3931093, 18, "xcvu190", None),
        info(0x3933093, 18, "xcvu160", None),
        info(0x3939093, 6, "xcvu065", Some(6271903)),
        info(0x393b093, 6, "xcvu065_CIV", Some(6271903)),
        info(0x396d093, 18, "xcvu440", None),
        info(0x396f093, 18, "xcvu440_CIV", None),
        info(0x4826093, 6, "xczu1cg", Some(742258)),
        info(0x484a093, 6, "xcku9p", Some(6627298)),
        info(0x484b093, 6, "xczu6cg", Some(6627298)),
        info(0x4a42093, 6, "xczu3cg", Some(1391770)),
        info(0x4a43093, 6, "xczu2cg", Some(1391770)),
        info(0x4a44093, 6, "xck24", Some(1391770)),
        info(0x4a46093, 6, "xczu5cg", Some(1949026)),
        info(0x4a47093, 6, "xczu4cg", Some(1949026)),
        info(0x4a49093, 6, "xck26", Some(1949026)),
        info(0x4a4e093, 6, "xcku11p", Some(5894830)),
        info(0x4a51093, 6, "xcku11p_CIV", Some(5894830)),
        info(0x4a52093, 6, "xcku13p", Some(7174789)),
        info(0x4a56093, 6, "xcku15p", Some(9085381)),
        info(0x4a57093, 6, "xczu17eg", Some(9085381)),
        info(0x4a59093, 6, "xcku15p_CIV", Some(9085381)),
        info(0x4a5a093, 6, "xczu7cg", Some(4827376)),
        info(0x4a5c093, 6, "xcu30", Some(4827376)),
        info(0x4a62093, 6, "xcku5p", Some(3857386)),
        info(0x4a63093, 6, "xcku3p", Some(3857386)),
        info(0x4a64093, 6, "xcau25p", Some(3857386)),
        info(0x4a65093, 6, "xcau20p", Some(3857386)),
        info(0x4a82093, 6, "xczu28dr", Some(8608942)),
        info(0x4a83093, 6, "xczu21dr", Some(8608942)),
        info(0x4a84093, 6, "xczu29dr", None),
        info(0x4a86093, 6, "xczu27dr", Some(8608942)),
        info(0x4a87093, 6, "xczu25dr", Some(8608942)),
        info(0x4a88093, 6, "xczu39dr", Some(8608942)),
        info(0x4aa2093, 6, "xczu46dr", Some(8608942)),
        info(0x4aa5093, 6, "xqzu48dr", Some(8608942)),
        info(0x4aa7093, 6, "xczu43dr", Some(8608942)),
        info(0x4aa8093, 6, "xqzu49dr", Some(8608942)),
        info(0x4aa9093, 6, "xczu47dr", Some(8608942)),
        info(0x4ac2093, 6, "xcau15p", Some(1337086)),
        info(0x4ac4093, 6, "xcau10p", Some(1337086)),
        info(0x4ace093, 6, "xcvu23p", Some(16310830)),
        info(0x4acf093, 6, "xcku19p", Some(16310830)),
        info(0x4ad3093, 6, "xcku19p_CIV", Some(16310830)),
        info(0x4ad4093, 6, "xcvu23p_CIV", Some(16310830)),
        info(0x4ad5093, 6, "xcu26", Some(16310830)),
        info(0x4ad6093, 6, "xczu67dr", Some(5214628)),
        info(0x4ad7093, 6, "xczu65dr", Some(5214628)),
        info(0x4ada093, 6, "xczu42dr", Some(5214628)),
        info(0x4adb093, 6, "xczu63dr", Some(5214628)),
        info(0x4adc093, 6, "xczu64dr", Some(5214628)),
        info(0x4af2093, 6, "xczu3tcg", Some(1301653)),
        info(0x4af6093, 6, "xcau7p", Some(767926)),
        info(0x4b29093, 12, "xcvu7p", None),
        info(0x4b2b093, 12, "xcvu5p", None),
        info(0x4b2d093, 12, "xcvu7p_CIV", None),
        info(0x4b2f093, 12, "xcvu5p_CIV", None),
        info(0x4b31093, 18, "xcvu9p", None),
        info(0x4b35093, 18, "xcvu9p_CIV", None),
        info(0x4b37093, 18, "xcu200", None),
        info(0x4b39093, 6, "xcvu3p", Some(6679378)),
        info(0x4b3d093, 6, "xcvu3p_CIV", Some(6679378)),
        info(0x4b41093, 24, "xcvu29p", None),
        info(0x4b43093, 24, "xcvu27p", None),
        info(0x4b45093, 24, "xcvu29p_CIV", None),
        info(0x4b49093, 18, "xcvu11p", None),
        info(0x4b4f093, 18, "xcvu11p_CIV", None),
        info(0x4b51093, 24, "xcvu13p", None),
        info(0x4b55093, 24, "xcvu13p_CIV", None),
        info(0x4b61093, 18, "xcvu57p", None),
        info(0x4b65093, 18, "xcvu57p_CIV", None),
        info(0x4b69093, 6, "xcvu33p", Some(7081882)),
        info(0x4b6b093, 6, "xcvu31p", Some(7081882)),
        info(0x4b6d093, 6, "xcvu33p_CIV", Some(7081882)),
        info(0x4b6f093, 6, "xcvu31p_CIV", Some(7081882)),
        info(0x4b71093, 12, "xcvu35p", None),
        info(0x4b73093, 12, "xcvu45p", None),
        info(0x4b75093, 12, "xcvu45p_CIV", None),
        info(0x4b77093, 12, "xcu50", None),
        info(0x4b79093, 18, "xcvu37p", None),
        info(0x4b7b093, 18, "xcvu47p", None),
        info(0x4b7d093, 18, "xcu55c", None),
        info(0x4ba1093, 24, "xcvu19p", None),
        info(0x4ba5093, 24, "xcvu19p_CIV", None),
        info(0x4e80093, 6, "xcsu35p", Some(331570)),
        info(0x4e81093, 6, "xcsu10p", Some(331570)),
        info(0x4e82093, 6, "xcsu25p", Some(331570)),
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

fn xilinx_versal() -> impl Iterator<Item = (IdCode, DeviceInfo)> {
    const fn info(idcode: u32, irlen: u8, name: &'static str) -> (IdCode, DeviceInfo) {
        let specific = Specific::XilinxVersal(XilinxVersalInfo {});
        let info = DeviceInfo {
            irlen: Bits(irlen),
            name,
            specific,
        };
        assert!(irlen.is_multiple_of(6));
        (id(idcode), info)
    }

    static DEVICES: &[(IdCode, DeviceInfo)] = &[
        info(0x4c08093, 6, "xqvm1402"),
        info(0x4c09093, 6, "xcvm1302"),
        info(0x4c18093, 6, "xcvp1052"),
        info(0x4c1b093, 6, "xcvp1002"),
        info(0x4c20093, 6, "xcvp1402"),
        info(0x4c22093, 6, "xcvp1102"),
        info(0x4c23093, 6, "xcvm2902"),
        info(0x4c24093, 6, "xcvm2302"),
        info(0x4c40093, 24, "xcvp1902"),
        info(0x4c60093, 6, "xcvm2152"),
        info(0x4c98093, 6, "xcvc1702"),
        info(0x4c99093, 6, "xqvm1502"),
        info(0x4c9a093, 6, "xcve1752"),
        info(0x4c9b093, 6, "xcvc1502"),
        info(0x4ca8093, 6, "xcvc1902"),
        info(0x4ca9093, 6, "xcvc1802"),
        info(0x4caa093, 6, "xqvm1802"),
        info(0x4cc0093, 6, "xcve2102"),
        info(0x4cc1093, 6, "xcve2002"),
        info(0x4cc8093, 6, "xcve2302"),
        info(0x4cc9093, 6, "xcve2202"),
        info(0x4cca093, 6, "xqvm1102"),
        info(0x4cd0093, 6, "xcvc2802"),
        info(0x4cd1093, 6, "xcvc2602"),
        info(0x4cd2093, 6, "xcve2602"),
        info(0x4cd3093, 6, "xcve2802"),
        info(0x4cd4093, 6, "xcvm2202"),
        info(0x4d00093, 6, "xcvp1202"),
        info(0x4d01093, 6, "xcvm2502"),
        info(0x4d08093, 12, "xcvp1502"),
        info(0x4d10093, 18, "xcvp1702"),
        info(0x4d14093, 24, "xcvp1802"),
        info(0x4d1c093, 12, "xcvp2502"),
        info(0x4d20093, 24, "xcvp2802"),
        info(0x4d28093, 12, "xcvh1582"),
        info(0x4d29093, 12, "xcvh1542"),
        info(0x4d2a093, 12, "xcvh1522"),
        info(0x4d2c093, 18, "xcvh1782"),
        info(0x4d2d093, 18, "xcvh1742"),
        info(0x4d2f093, 18, "xcv80"),
        info(0x4d34093, 12, "xcvp1552"),
        info(0x4da9093, 6, "xc2ve3558"),
        info(0x4dab093, 6, "xc2ve3504"),
        info(0x4dac093, 6, "xc2vm3558"),
        info(0x4db0093, 6, "xc2ve3858"),
        info(0x4db1093, 6, "xc2ve3804"),
        info(0x4db2093, 6, "xc2vm3858"),
    ];

    DEVICES.iter().cloned()
}
