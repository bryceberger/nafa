use crate::units::{Bits, Words32};

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IdCode(u32);

#[derive(Clone)]
pub struct DeviceInfo {
    pub irlen: Bits<u8>,
    pub name: &'static str,
    pub specific: Specific,
}

#[derive(Clone)]
pub enum Specific {
    Unknown,
    Xilinx32(Xilinx32Info),
}

#[derive(Clone)]
pub struct Xilinx32Info {
    pub family: Xilinx32Family,
    pub slr: u8,
    pub readback: Words32<usize>,
}

#[derive(Clone, Copy)]
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
    Versal,
}

impl IdCode {
    pub const fn new(code: u32) -> Self {
        /// IEEE 11491-2013, Figure 12-1, "Structure of the device
        /// identification code"
        const VERSION: u32 = 0xf0000000;
        Self(code & !VERSION)
    }

    pub const fn code(self) -> u32 {
        self.0
    }
}

impl From<u32> for IdCode {
    fn from(value: u32) -> Self {
        Self::new(value)
    }
}

impl From<IdCode> for u32 {
    fn from(val: IdCode) -> Self {
        val.code()
    }
}

/// Returns iterator of `(idcode, info)`. Intended to be collected into a
/// `HashMap`, to be passed to [`crate::Controller::new`].
pub fn builtin() -> impl Iterator<Item = (IdCode, DeviceInfo)> {
    xilinx()
}

fn xilinx() -> impl Iterator<Item = (IdCode, DeviceInfo)> {
    use Bits as B;
    use Specific as S;
    use Words32 as W;
    use Xilinx32Family as F;
    use Xilinx32Info as X;

    const fn id(code: u32) -> IdCode {
        IdCode::new(code)
    }

    #[rustfmt::skip]
    static DEVICES: &[(IdCode, DeviceInfo)] = &[
        // Kintex US (6 IR_Len)
        (id(0x03824093), DeviceInfo { irlen: B( 6), name: "XCKU025",        specific: S::Xilinx32(X { slr: 1, readback: W( 4001190), family: F::US }) }), //id_cmd: 0x09,
        (id(0x03823093), DeviceInfo { irlen: B( 6), name: "XCKU035",        specific: S::Xilinx32(X { slr: 1, readback: W( 4001190), family: F::US }) }), //id_cmd: 0x09,
        (id(0x03822093), DeviceInfo { irlen: B( 6), name: "XCKU040",        specific: S::Xilinx32(X { slr: 1, readback: W( 4001190), family: F::US }) }), //id_cmd: 0x09,
        (id(0x03919093), DeviceInfo { irlen: B( 6), name: "XCKU060",        specific: S::Xilinx32(X { slr: 1, readback: W( 6030690), family: F::US }) }), //id_cmd: 0x09,
        (id(0x03844093), DeviceInfo { irlen: B( 6), name: "XCKU095",        specific: S::Xilinx32(X { slr: 1, readback: W( 8960304), family: F::US }) }), //id_cmd: 0x09,
        // Kintex US (12 IR_Len)
        (id(0x0380F093), DeviceInfo { irlen: B(12), name: "XCKU085",        specific: S::Xilinx32(X { slr: 2, readback: W(12061380), family: F::US }) }), //id_cmd: 0x09,
        (id(0x0390D093), DeviceInfo { irlen: B(12), name: "XCKU115",        specific: S::Xilinx32(X { slr: 2, readback: W(12061380), family: F::US }) }), //id_cmd: 0x09,
        // Virtex US (6 IR_Len)
        (id(0x03939093), DeviceInfo { irlen: B( 6), name: "XCVU065",        specific: S::Xilinx32(X { slr: 1, readback: W( 6271770), family: F::US }) }), //id_cmd: 0x09,
        (id(0x03843093), DeviceInfo { irlen: B( 6), name: "XCVU080",        specific: S::Xilinx32(X { slr: 1, readback: W( 8960304), family: F::US }) }), //id_cmd: 0x09,
        (id(0x03842093), DeviceInfo { irlen: B( 6), name: "XCVU095",        specific: S::Xilinx32(X { slr: 1, readback: W( 8960304), family: F::US }) }), //id_cmd: 0x09,
        // Virtex US (12 IR_Len)
        (id(0x0392D093), DeviceInfo { irlen: B(12), name: "XCVU125",        specific: S::Xilinx32(X { slr: 2, readback: W(12543540), family: F::US }) }), //id_cmd: 0x09,
        // Virtex US (18 IR_Len)
        (id(0x03933093), DeviceInfo { irlen: B(18), name: "XCVU160",        specific: S::Xilinx32(X { slr: 3, readback: W(18815310), family: F::US }) }), //id_cmd: 0x09,
        (id(0x03931093), DeviceInfo { irlen: B(18), name: "XCVU190",        specific: S::Xilinx32(X { slr: 3, readback: W(18815310), family: F::US }) }), //id_cmd: 0x09,
        (id(0x0396D093), DeviceInfo { irlen: B(18), name: "XCVU440",        specific: S::Xilinx32(X { slr: 3, readback: W(32239530), family: F::US }) }), //id_cmd: 0x09,
        // Spartan US+ (6 IR_Len)
        (id(0x04E81093), DeviceInfo { irlen: B( 6), name: "SU10P",          specific: S::Xilinx32(X { slr: 1, readback: W(10606464), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04E82093), DeviceInfo { irlen: B( 6), name: "SU25P",          specific: S::Xilinx32(X { slr: 1, readback: W(10606464), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04E80093), DeviceInfo { irlen: B( 6), name: "SU35P",          specific: S::Xilinx32(X { slr: 1, readback: W(10606464), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04E88093), DeviceInfo { irlen: B( 6), name: "SU50P",          specific: S::Xilinx32(X { slr: 1, readback: W(15728640), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04E90093), DeviceInfo { irlen: B( 6), name: "SU55P",          specific: S::Xilinx32(X { slr: 1, readback: W(15728640), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04E99093), DeviceInfo { irlen: B( 6), name: "SU65P",          specific: S::Xilinx32(X { slr: 1, readback: W(       0), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04E98093), DeviceInfo { irlen: B( 6), name: "SU100P",         specific: S::Xilinx32(X { slr: 1, readback: W(29360128), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04EA1093), DeviceInfo { irlen: B( 6), name: "SU150P",         specific: S::Xilinx32(X { slr: 1, readback: W(57817728), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04EA0093), DeviceInfo { irlen: B( 6), name: "SU200P",         specific: S::Xilinx32(X { slr: 1, readback: W(57817728), family: F::UP }) }), //id_cmd: 0x09,
        // Artix US+
        (id(0x04AF6093), DeviceInfo { irlen: B( 6), name: "XCAU7P",         specific: S::Xilinx32(X { slr: 1, readback: W(  767808), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04AC4093), DeviceInfo { irlen: B( 6), name: "XCAU10P",        specific: S::Xilinx32(X { slr: 1, readback: W( 1336968), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04AC2093), DeviceInfo { irlen: B( 6), name: "XCAU15P",        specific: S::Xilinx32(X { slr: 1, readback: W( 1336968), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04A65093), DeviceInfo { irlen: B( 6), name: "XCAU20P",        specific: S::Xilinx32(X { slr: 1, readback: W( 3857268), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04A64093), DeviceInfo { irlen: B( 6), name: "XCAU25P",        specific: S::Xilinx32(X { slr: 1, readback: W( 3857268), family: F::UP }) }), //id_cmd: 0x09,
        // Kintex US+
        (id(0x04A63093), DeviceInfo { irlen: B( 6), name: "XCKU3P",         specific: S::Xilinx32(X { slr: 1, readback: W( 3857268), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04A62093), DeviceInfo { irlen: B( 6), name: "XCKU5P",         specific: S::Xilinx32(X { slr: 1, readback: W( 3857268), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x0484A093), DeviceInfo { irlen: B( 6), name: "XCKU9P",         specific: S::Xilinx32(X { slr: 1, readback: W( 6627180), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04A4E093), DeviceInfo { irlen: B( 6), name: "XCKU11P",        specific: S::Xilinx32(X { slr: 1, readback: W( 5894712), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04A52093), DeviceInfo { irlen: B( 6), name: "XCKU13P",        specific: S::Xilinx32(X { slr: 1, readback: W( 7174671), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04A56093), DeviceInfo { irlen: B( 6), name: "XCKU15P",        specific: S::Xilinx32(X { slr: 1, readback: W( 9085263), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04ACF093), DeviceInfo { irlen: B( 6), name: "XCKU19P",        specific: S::Xilinx32(X { slr: 1, readback: W(16310712), family: F::UP }) }), //id_cmd: 0x09,
        // Virtex US+ (6 IR_len)
        (id(0x04B39093), DeviceInfo { irlen: B( 6), name: "XCVU3P",         specific: S::Xilinx32(X { slr: 1, readback: W( 6679260), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04ACE093), DeviceInfo { irlen: B( 6), name: "XCVU23P",        specific: S::Xilinx32(X { slr: 1, readback: W(16310712), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04B6B093), DeviceInfo { irlen: B( 6), name: "XCVU31P",        specific: S::Xilinx32(X { slr: 1, readback: W( 7081764), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04B69093), DeviceInfo { irlen: B( 6), name: "XCVU33P",        specific: S::Xilinx32(X { slr: 1, readback: W( 7081764), family: F::UP }) }), //id_cmd: 0x09,
        // Virtex US+ (12 IR_len)
        (id(0x04B2B093), DeviceInfo { irlen: B(12), name: "XCVU5P",         specific: S::Xilinx32(X { slr: 2, readback: W(13358520), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04B29093), DeviceInfo { irlen: B(12), name: "XCVU7P",         specific: S::Xilinx32(X { slr: 2, readback: W(13358520), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04B71093), DeviceInfo { irlen: B(12), name: "XCVU35P",        specific: S::Xilinx32(X { slr: 2, readback: W(14163528), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04B73093), DeviceInfo { irlen: B(12), name: "XCVU45P",        specific: S::Xilinx32(X { slr: 2, readback: W(14163528), family: F::UP }) }), //id_cmd: 0x09,
        // Virtex US+ (18 IR_len)
        (id(0x14B31093), DeviceInfo { irlen: B(18), name: "XCVU9P",         specific: S::Xilinx32(X { slr: 3, readback: W(20035783), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x14B49093), DeviceInfo { irlen: B(18), name: "XCVU11P",        specific: S::Xilinx32(X { slr: 3, readback: W(21245292), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04B79093), DeviceInfo { irlen: B(18), name: "XCVU37P",        specific: S::Xilinx32(X { slr: 3, readback: W(21245292), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04B7B093), DeviceInfo { irlen: B(18), name: "XCVU47P",        specific: S::Xilinx32(X { slr: 3, readback: W(21245292), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04B61093), DeviceInfo { irlen: B(18), name: "XCVU57P",        specific: S::Xilinx32(X { slr: 3, readback: W(21245292), family: F::UP }) }), //id_cmd: 0x09,
        // Virtex US+ (24 IR_len)
        (id(0x04B51093), DeviceInfo { irlen: B(24), name: "XCVU13P",        specific: S::Xilinx32(X { slr: 4, readback: W(28327056), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04BA1093), DeviceInfo { irlen: B(24), name: "XCVU19P",        specific: S::Xilinx32(X { slr: 4, readback: W(49775460), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04B43093), DeviceInfo { irlen: B(24), name: "XCVU27P",        specific: S::Xilinx32(X { slr: 4, readback: W(28327056), family: F::UP }) }), //id_cmd: 0x09,
        (id(0x04B41093), DeviceInfo { irlen: B(24), name: "XCVU29P",        specific: S::Xilinx32(X { slr: 4, readback: W(28327056), family: F::UP }) }), //id_cmd: 0x09,
        // Zynq US+
        (id(0x04688093), DeviceInfo { irlen: B(12), name: "XCZU1EG",        specific: S::Xilinx32(X { slr: 1, readback: W(  742140), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x14711093), DeviceInfo { irlen: B(12), name: "XCZU2EG",        specific: S::Xilinx32(X { slr: 1, readback: W( 1391652), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x14710093), DeviceInfo { irlen: B(12), name: "XCZU3EG",        specific: S::Xilinx32(X { slr: 1, readback: W( 1391652), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x04721093), DeviceInfo { irlen: B(12), name: "XCZU4EG",        specific: S::Xilinx32(X { slr: 1, readback: W( 1948939), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x04720093), DeviceInfo { irlen: B(12), name: "XCZU5EG",        specific: S::Xilinx32(X { slr: 1, readback: W( 1948939), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x24739093), DeviceInfo { irlen: B(12), name: "XCZU6EG",        specific: S::Xilinx32(X { slr: 1, readback: W( 6627180), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x14730093), DeviceInfo { irlen: B(12), name: "XCZU7EG",        specific: S::Xilinx32(X { slr: 1, readback: W( 4827258), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x24738093), DeviceInfo { irlen: B(12), name: "XCZU9EG",        specific: S::Xilinx32(X { slr: 1, readback: W( 6627180), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x04740093), DeviceInfo { irlen: B(12), name: "XCZU11EG",       specific: S::Xilinx32(X { slr: 1, readback: W( 5894712), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x14750093), DeviceInfo { irlen: B(12), name: "XCZU15EG",       specific: S::Xilinx32(X { slr: 1, readback: W( 7174671), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x14759093), DeviceInfo { irlen: B(12), name: "XCZU17EG",       specific: S::Xilinx32(X { slr: 1, readback: W( 9085263), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x14758093), DeviceInfo { irlen: B(12), name: "XCZU19EG",       specific: S::Xilinx32(X { slr: 1, readback: W( 9085263), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x147E1093), DeviceInfo { irlen: B(12), name: "XCZU21DR",       specific: S::Xilinx32(X { slr: 1, readback: W( 8608824), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x147E5093), DeviceInfo { irlen: B(12), name: "XCZU25DR",       specific: S::Xilinx32(X { slr: 1, readback: W( 8608824), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x147E4093), DeviceInfo { irlen: B(12), name: "XCZU27DR",       specific: S::Xilinx32(X { slr: 1, readback: W( 8608824), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x147E0093), DeviceInfo { irlen: B(12), name: "XCZU28DR",       specific: S::Xilinx32(X { slr: 1, readback: W( 8608824), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x147E2093), DeviceInfo { irlen: B(12), name: "XCZU29DR",       specific: S::Xilinx32(X { slr: 1, readback: W( 8608824), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x147E6093), DeviceInfo { irlen: B(12), name: "XCZU39DR",       specific: S::Xilinx32(X { slr: 1, readback: W( 8608824), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x046D4093), DeviceInfo { irlen: B(12), name: "XCZU42DR",       specific: S::Xilinx32(X { slr: 1, readback: W( 5214510), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x147FD093), DeviceInfo { irlen: B(12), name: "XCZU43DR",       specific: S::Xilinx32(X { slr: 1, readback: W( 8608824), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x147F8093), DeviceInfo { irlen: B(12), name: "XCZU46DR",       specific: S::Xilinx32(X { slr: 1, readback: W( 8608824), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x147FF093), DeviceInfo { irlen: B(12), name: "XCZU47DR",       specific: S::Xilinx32(X { slr: 1, readback: W( 8608824), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x147FB093), DeviceInfo { irlen: B(12), name: "XCZU48DR",       specific: S::Xilinx32(X { slr: 1, readback: W( 8608824), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x147FE093), DeviceInfo { irlen: B(12), name: "XCZU49DR",       specific: S::Xilinx32(X { slr: 1, readback: W( 8608824), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x046D5093), DeviceInfo { irlen: B(12), name: "XCZU63DR",       specific: S::Xilinx32(X { slr: 1, readback: W( 5214510), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x046D6093), DeviceInfo { irlen: B(12), name: "XCZU64DR",       specific: S::Xilinx32(X { slr: 1, readback: W( 5214510), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x046D1093), DeviceInfo { irlen: B(12), name: "XCZU65DR",       specific: S::Xilinx32(X { slr: 1, readback: W( 5214510), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x046D0093), DeviceInfo { irlen: B(12), name: "XCZU67DR",       specific: S::Xilinx32(X { slr: 1, readback: W( 5214510), family: F::ZP }) }), //id_cmd: 0x09,
        (id(0x04718093), DeviceInfo { irlen: B(12), name: "XCZU3TEG",       specific: S::Xilinx32(X { slr: 1, readback: W( 1301535), family: F::ZP }) }), //id_cmd: 0x09,
        // Versal
        (id(0x14D00093), DeviceInfo { irlen: B( 6), name: "XCVP1202",       specific: S::Xilinx32(X { slr: 1, readback: W(       0), family: F::Versal }) }), //id_cmd: 0x09,
        // Spartan-7
        (id(0x03622093), DeviceInfo { irlen: B( 6), name: "XC7S6",          specific: S::Xilinx32(X { slr: 1, readback: W(  134711), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x03620093), DeviceInfo { irlen: B( 6), name: "XC7S15",         specific: S::Xilinx32(X { slr: 1, readback: W(  134711), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x037C4093), DeviceInfo { irlen: B( 6), name: "XC7S25",         specific: S::Xilinx32(X { slr: 1, readback: W(  310451), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x0362F093), DeviceInfo { irlen: B( 6), name: "XC7S50",         specific: S::Xilinx32(X { slr: 1, readback: W(  548003), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x037C8093), DeviceInfo { irlen: B( 6), name: "XC7S75",         specific: S::Xilinx32(X { slr: 1, readback: W(  921703), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x037C7093), DeviceInfo { irlen: B( 6), name: "XC7S100",        specific: S::Xilinx32(X { slr: 1, readback: W(  921703), family: F::S7 }) }), //id_cmd: 0x09,
        // Artix-7
        (id(0x037C3093), DeviceInfo { irlen: B( 6), name: "XC7A12T",        specific: S::Xilinx32(X { slr: 1, readback: W(  310451), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x0362E093), DeviceInfo { irlen: B( 6), name: "XC7A15T",        specific: S::Xilinx32(X { slr: 1, readback: W(  548003), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x037C2093), DeviceInfo { irlen: B( 6), name: "XC7A25T",        specific: S::Xilinx32(X { slr: 1, readback: W(  310451), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x0362D093), DeviceInfo { irlen: B( 6), name: "XC7A35T",        specific: S::Xilinx32(X { slr: 1, readback: W(  548003), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x0362C093), DeviceInfo { irlen: B( 6), name: "XC7A50T",        specific: S::Xilinx32(X { slr: 1, readback: W(  548003), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x03632093), DeviceInfo { irlen: B( 6), name: "XC7A75T",        specific: S::Xilinx32(X { slr: 1, readback: W(  956447), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x03631093), DeviceInfo { irlen: B( 6), name: "XC7A100T",       specific: S::Xilinx32(X { slr: 1, readback: W(  956447), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x03636093), DeviceInfo { irlen: B( 6), name: "XC7A200T",       specific: S::Xilinx32(X { slr: 1, readback: W( 2432663), family: F::S7 }) }), //id_cmd: 0x09,
        // Kintex-7
        (id(0x03642093), DeviceInfo { irlen: B( 6), name: "XC7K30T",        specific: S::Xilinx32(X { slr: 1, readback: W(       0), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x03647093), DeviceInfo { irlen: B( 6), name: "XC7K70T",        specific: S::Xilinx32(X { slr: 1, readback: W(  752831), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x0364C093), DeviceInfo { irlen: B( 6), name: "XC7K160T",       specific: S::Xilinx32(X { slr: 1, readback: W( 1673143), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x03651093), DeviceInfo { irlen: B( 6), name: "XC7K325T",       specific: S::Xilinx32(X { slr: 1, readback: W( 2860903), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x03747093), DeviceInfo { irlen: B( 6), name: "XC7K355T",       specific: S::Xilinx32(X { slr: 1, readback: W( 3512959), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x03656093), DeviceInfo { irlen: B( 6), name: "XC7K410T",       specific: S::Xilinx32(X { slr: 1, readback: W( 3969479), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x03752093), DeviceInfo { irlen: B( 6), name: "XC7K420T",       specific: S::Xilinx32(X { slr: 1, readback: W( 4683751), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x03751093), DeviceInfo { irlen: B( 6), name: "XC7K480T",       specific: S::Xilinx32(X { slr: 1, readback: W( 4683751), family: F::S7 }) }), //id_cmd: 0x09,
        // Virtex-7
        (id(0x03671093), DeviceInfo { irlen: B( 6), name: "XC7V585T",       specific: S::Xilinx32(X { slr: 1, readback: W( 5043715), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x036B3093), DeviceInfo { irlen: B(24), name: "XC7V2000T",      specific: S::Xilinx32(X { slr: 4, readback: W(13979288), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x03667093), DeviceInfo { irlen: B( 6), name: "XC7VX330T",      specific: S::Xilinx32(X { slr: 1, readback: W( 3476195), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x03682093), DeviceInfo { irlen: B( 6), name: "XC7VX415T",      specific: S::Xilinx32(X { slr: 1, readback: W( 4310455), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x03687093), DeviceInfo { irlen: B( 6), name: "XC7VX485T",      specific: S::Xilinx32(X { slr: 1, readback: W( 5068359), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x03692093), DeviceInfo { irlen: B( 6), name: "XC7VX550T",      specific: S::Xilinx32(X { slr: 1, readback: W( 7183703), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x03691093), DeviceInfo { irlen: B( 6), name: "XC7VX690T",      specific: S::Xilinx32(X { slr: 1, readback: W( 7183703), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x03696093), DeviceInfo { irlen: B( 6), name: "XC7VX980T",      specific: S::Xilinx32(X { slr: 1, readback: W( 8828791), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x036D5093), DeviceInfo { irlen: B(24), name: "XC7VX1140T",     specific: S::Xilinx32(X { slr: 4, readback: W(12035240), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x036D9093), DeviceInfo { irlen: B(22), name: "XC7VH580T",      specific: S::Xilinx32(X { slr: 3, readback: W( 6114469), family: F::S7 }) }), //id_cmd: 0x09,
        (id(0x036DB093), DeviceInfo { irlen: B(38), name: "XC7VH870T",      specific: S::Xilinx32(X { slr: 5, readback: W( 9187698), family: F::S7 }) }), //id_cmd: 0x09,
        // Zynq-7
        (id(0x03722093), DeviceInfo { irlen: B( 6), name: "XC7Z010",        specific: S::Xilinx32(X { slr: 1, readback: W(  520935), family: F::Z7 }) }), //id_cmd: 0x09,
        (id(0x03727093), DeviceInfo { irlen: B( 6), name: "XC7Z020",        specific: S::Xilinx32(X { slr: 1, readback: W( 1011391), family: F::Z7 }) }), //id_cmd: 0x09,
        (id(0x0372C093), DeviceInfo { irlen: B( 6), name: "XC7Z030",        specific: S::Xilinx32(X { slr: 1, readback: W( 1494979), family: F::Z7 }) }), //id_cmd: 0x09,
        (id(0x03731093), DeviceInfo { irlen: B( 6), name: "XC7Z045",        specific: S::Xilinx32(X { slr: 1, readback: W( 3330351), family: F::Z7 }) }), //id_cmd: 0x09,
        (id(0x03736093), DeviceInfo { irlen: B( 6), name: "XC7Z100",        specific: S::Xilinx32(X { slr: 1, readback: W( 4354087), family: F::Z7 }) }), //id_cmd: 0x09,
        // Spartan-6
        (id(0x04000093), DeviceInfo { irlen: B( 6), name: "XC6SLX4",        specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   85359, slr: 1, family: Series6, 
        (id(0x04001093), DeviceInfo { irlen: B( 6), name: "XC6SLX9",        specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   85704, slr: 1, family: Series6, 
        (id(0x04002093), DeviceInfo { irlen: B( 6), name: "XC6SLX16",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  116602, slr: 1, family: Series6, 
        (id(0x04004093), DeviceInfo { irlen: B( 6), name: "XC6SLX25",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  201264, slr: 1, family: Series6, 
        (id(0x04024093), DeviceInfo { irlen: B( 6), name: "XC6SLX25T",      specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  201264, slr: 1, family: Series6, 
        (id(0x04008093), DeviceInfo { irlen: B( 6), name: "XC6SLX45",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  373103, slr: 1, family: Series6, 
        (id(0x04028093), DeviceInfo { irlen: B( 6), name: "XC6SLX45T",      specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  373103, slr: 1, family: Series6, 
        (id(0x0400E093), DeviceInfo { irlen: B( 6), name: "XC6SLX75",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  616241, slr: 1, family: Series6-EX, 
        (id(0x0402E093), DeviceInfo { irlen: B( 6), name: "XC6SLX75T",      specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  616241, slr: 1, family: Series6-EX, 
        (id(0x04011093), DeviceInfo { irlen: B( 6), name: "XC6SLX100",      specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  834101, slr: 1, family: Series6-EX, 
        (id(0x04031093), DeviceInfo { irlen: B( 6), name: "XC6SLX100T",     specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  834101, slr: 1, family: Series6-EX, 
        (id(0x0401D093), DeviceInfo { irlen: B( 6), name: "XC6SLX150",      specific: S::Unknown }), // id_cmd: 0x09,  readback_bits: 1059677, slr: 1, family: Series6-EX, 
        (id(0x0403D093), DeviceInfo { irlen: B( 6), name: "XC6SLX150T",     specific: S::Unknown }), // id_cmd: 0x09,  readback_bits: 1059677, slr: 1, family: Series6-EX, 
        // Virtex-6
        (id(0x042A2093), DeviceInfo { irlen: B(10), name: "XC6HX250T",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 2495707, slr: 1, family: Virtex6, 
        (id(0x042A4093), DeviceInfo { irlen: B(10), name: "XC6HX255T",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 2495707, slr: 1, family: Virtex6, 
        (id(0x042A8093), DeviceInfo { irlen: B(10), name: "XC6HX380T",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 3743269, slr: 1, family: Virtex6, 
        (id(0x042AC093), DeviceInfo { irlen: B(10), name: "XC6HX565T",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 5020477, slr: 1, family: Virtex6, 
        (id(0x04244093), DeviceInfo { irlen: B(10), name: "XC6VLX75T",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  819979, slr: 1, family: Virtex6, 
        (id(0x0424A093), DeviceInfo { irlen: B(10), name: "XC6VLX130T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 1366243, slr: 1, family: Virtex6, 
        (id(0x0424C093), DeviceInfo { irlen: B(10), name: "XC6VLX195T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 1923523, slr: 1, family: Virtex6, 
        (id(0x04250093), DeviceInfo { irlen: B(10), name: "XC6VLX240T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 2308111, slr: 1, family: Virtex6, 
        (id(0x04252093), DeviceInfo { irlen: B(10), name: "XC6VLX365T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 3002119, slr: 1, family: Virtex6, 
        (id(0x04256093), DeviceInfo { irlen: B(10), name: "XC6VLX550T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 4502887, slr: 1, family: Virtex6, 
        (id(0x0423A093), DeviceInfo { irlen: B(10), name: "XC6VLX760",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 5775721, slr: 1, family: Virtex6, 
        (id(0x04286093), DeviceInfo { irlen: B(10), name: "XC6VSX315T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 3264559, slr: 1, family: Virtex6, 
        (id(0x04288093), DeviceInfo { irlen: B(10), name: "XC6VSX475T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 4896547, slr: 1, family: Virtex6, 
        (id(0x042C4093), DeviceInfo { irlen: B(10), name: "XC6VCX75T",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:       0, slr: 1, family: Virtex6,
        (id(0x042CA093), DeviceInfo { irlen: B(10), name: "XC6VCX130T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:       0, slr: 1, family: Virtex6,
        (id(0x042CC093), DeviceInfo { irlen: B(10), name: "XC6VCX195T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:       0, slr: 1, family: Virtex6,
        (id(0x042D0093), DeviceInfo { irlen: B(10), name: "XC6VCX240T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:       0, slr: 1, family: Virtex6,
        // Virtex-5
        (id(0x0286E093), DeviceInfo { irlen: B(10), name: "XC5VLX30",       specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  261416, slr: 1, family: Virtex5, 
        (id(0x02896093), DeviceInfo { irlen: B(10), name: "XC5VLX50",       specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  392124, slr: 1, family: Virtex5, 
        (id(0x028AE093), DeviceInfo { irlen: B(10), name: "XC5VLX85",       specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  682404, slr: 1, family: Virtex5, 
        (id(0x028D6093), DeviceInfo { irlen: B(10), name: "XC5VLX110",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  909872, slr: 1, family: Virtex5, 
        (id(0x028EC093), DeviceInfo { irlen: B(10), name: "XC5VLX155",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 1334304, slr: 1, family: Virtex5, 
        (id(0x0290C093), DeviceInfo { irlen: B(10), name: "XC5VLX220",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 1660336, slr: 1, family: Virtex5, 
        (id(0x0295C095), DeviceInfo { irlen: B(10), name: "XC5VLX330",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 2490504, slr: 1, family: Virtex5, 
        (id(0x02A56093), DeviceInfo { irlen: B(10), name: "XC5VLX20T",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  154242, slr: 1, family: Virtex5, 
        (id(0x02A6E093), DeviceInfo { irlen: B(10), name: "XC5VLX30T",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  292576, slr: 1, family: Virtex5, 
        (id(0x02A96093), DeviceInfo { irlen: B(10), name: "XC5VLX50T",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  438864, slr: 1, family: Virtex5, 
        (id(0x02AAE093), DeviceInfo { irlen: B(10), name: "XC5VLX85T",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  729144, slr: 1, family: Virtex5, 
        (id(0x02AD6093), DeviceInfo { irlen: B(10), name: "XC5VLX110T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  972192, slr: 1, family: Virtex5, 
        (id(0x02AEC093), DeviceInfo { irlen: B(10), name: "XC5VLX155T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 1344800, slr: 1, family: Virtex5, 
        (id(0x02B0C093), DeviceInfo { irlen: B(10), name: "XC5VLX220T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 1722656, slr: 1, family: Virtex5, 
        (id(0x02B5C093), DeviceInfo { irlen: B(10), name: "XC5VLX330T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 2583984, slr: 1, family: Virtex5, 
        (id(0x04502093), DeviceInfo { irlen: B(10), name: "XC5VTX150T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 1352180, slr: 1, family: Virtex5, 
        (id(0x0453E093), DeviceInfo { irlen: B(10), name: "XC5VTX240T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 2054592, slr: 1, family: Virtex5, 
        (id(0x03276093), DeviceInfo { irlen: B(10), name: "XC5VFX30T",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  422136, slr: 1, family: Virtex5, 
        (id(0x032C6093), DeviceInfo { irlen: B(10), name: "XC5VFX70T",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  844272, slr: 1, family: Virtex5, 
        (id(0x032D8093), DeviceInfo { irlen: B(14), name: "XC5VFX100T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 1230656, slr: 1, family: Virtex5, 
        (id(0x03300093), DeviceInfo { irlen: B(14), name: "XC5VFX130T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 1538320, slr: 1, family: Virtex5, 
        (id(0x03334093), DeviceInfo { irlen: B(14), name: "XC5VFX200T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 2214000, slr: 1, family: Virtex5, 
        (id(0x02E72093), DeviceInfo { irlen: B(10), name: "XC5VSX35T",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  416888, slr: 1, family: Virtex5, 
        (id(0x02E9A093), DeviceInfo { irlen: B(10), name: "XC5VSX50T",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  625332, slr: 1, family: Virtex5, 
        (id(0x02ECE093), DeviceInfo { irlen: B(10), name: "XC5VSX95T",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 1115856, slr: 1, family: Virtex5, 
        (id(0x02F3E093), DeviceInfo { irlen: B(10), name: "XC5VSX260T",     specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 2487552, slr: 1, family: Virtex5, 
        // Virtex-4
        (id(0x01658093), DeviceInfo { irlen: B(10), name: "XC4VLX15",       specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  147600, slr: 1, family: Virtex4, 
        (id(0x0167C093), DeviceInfo { irlen: B(10), name: "XC4VLX25",       specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  243048, slr: 1, family: Virtex4, 
        (id(0x016A4093), DeviceInfo { irlen: B(10), name: "XC4VLX40",       specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  381792, slr: 1, family: Virtex4, 
        (id(0x016B4093), DeviceInfo { irlen: B(10), name: "XC4VLX60",       specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  552352, slr: 1, family: Virtex4, 
        (id(0x016D8093), DeviceInfo { irlen: B(10), name: "XC4VLX80",       specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  726520, slr: 1, family: Virtex4, 
        (id(0x01700093), DeviceInfo { irlen: B(10), name: "XC4VLX100",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  958416, slr: 1, family: Virtex4, 
        (id(0x01718093), DeviceInfo { irlen: B(10), name: "XC4VLX160",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 1259520, slr: 1, family: Virtex4, 
        (id(0x01734093), DeviceInfo { irlen: B(10), name: "XC4VLX200",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 1603920, slr: 1, family: Virtex4, 
        (id(0x02068093), DeviceInfo { irlen: B(10), name: "XC4VSX25",       specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  284540, slr: 1, family: Virtex4, 
        (id(0x02088093), DeviceInfo { irlen: B(10), name: "XC4VSX35",       specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  426810, slr: 1, family: Virtex4, 
        (id(0x020B0093), DeviceInfo { irlen: B(10), name: "XC4VSX55",       specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  709464, slr: 1, family: Virtex4, 
        (id(0x01E58093), DeviceInfo { irlen: B(10), name: "XC4VFX12",       specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  147600, slr: 1, family: Virtex4, 
        (id(0x01E64093), DeviceInfo { irlen: B(10), name: "XC4VFX20",       specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  225008, slr: 1, family: Virtex4, 
        (id(0x01E8C093), DeviceInfo { irlen: B(14), name: "XC4VFX40",       specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  422136, slr: 1, family: Virtex4, 
        (id(0x01EB4093), DeviceInfo { irlen: B(14), name: "XC4VFX60",       specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits:  665016, slr: 1, family: Virtex4, 
        (id(0x01EE4093), DeviceInfo { irlen: B(14), name: "XC4VFX100",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 1031970, slr: 1, family: Virtex4, 
        (id(0x01f14093), DeviceInfo { irlen: B(14), name: "XC4VFX140",      specific: S::Unknown }), // id_cmd: 0x3c9, readback_bits: 1494204, slr: 1, family: Virtex4, 
        // Spartan-3
        (id(0x0140D093), DeviceInfo { irlen: B( 6), name: "XC3S50",         specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   13727, slr: 1, family: Spartan3, 
        (id(0x01414093), DeviceInfo { irlen: B( 6), name: "XC3S200",        specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   32738, slr: 1, family: Spartan3, 
        (id(0x0141C093), DeviceInfo { irlen: B( 6), name: "XC3S400",        specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   53098, slr: 1, family: Spartan3, 
        (id(0x01428093), DeviceInfo { irlen: B( 6), name: "XC3S1000",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  100734, slr: 1, family: Spartan3, 
        (id(0x01434093), DeviceInfo { irlen: B( 6), name: "XC3S1500",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  162962, slr: 1, family: Spartan3, 
        (id(0x01440093), DeviceInfo { irlen: B( 6), name: "XC3S2000",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  239782, slr: 1, family: Spartan3, 
        (id(0x01448093), DeviceInfo { irlen: B( 6), name: "XC3S4000",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  353652, slr: 1, family: Spartan3, 
        (id(0x01450093), DeviceInfo { irlen: B( 6), name: "XC3S5000",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  414748, slr: 1, family: Spartan3, 
        (id(0x01C10093), DeviceInfo { irlen: B( 6), name: "XC3S100E",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   18167, slr: 1, family: Spartan3, 
        (id(0x01C1A093), DeviceInfo { irlen: B( 6), name: "XC3S250E",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   42304, slr: 1, family: Spartan3, 
        (id(0x01C22093), DeviceInfo { irlen: B( 6), name: "XC3S500E",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   70944, slr: 1, family: Spartan3, 
        (id(0x01C2E093), DeviceInfo { irlen: B( 6), name: "XC3S1200E",      specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  120037, slr: 1, family: Spartan3, 
        (id(0x01C3A093), DeviceInfo { irlen: B( 6), name: "XC3S1600E",      specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  186553, slr: 1, family: Spartan3, 
        (id(0x02210093), DeviceInfo { irlen: B( 6), name: "XC3S50A",        specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   13666, slr: 1, family: Spartan3, 
        (id(0x02218093), DeviceInfo { irlen: B( 6), name: "XC3S200A",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   37379, slr: 1, family: Spartan3, 
        (id(0x02220093), DeviceInfo { irlen: B( 6), name: "XC3S400A",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   58955, slr: 1, family: Spartan3, 
        (id(0x02228093), DeviceInfo { irlen: B( 6), name: "XC3S700A",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   85395, slr: 1, family: Spartan3, 
        (id(0x02230093), DeviceInfo { irlen: B( 6), name: "XC3S1400A",      specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  148603, slr: 1, family: Spartan3, 
        (id(0x02610093), DeviceInfo { irlen: B( 6), name: "XC3S50AN",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   13666, slr: 1, family: Spartan3, 
        (id(0x02618093), DeviceInfo { irlen: B( 6), name: "XC3S200AN",      specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   37379, slr: 1, family: Spartan3, 
        (id(0x02620093), DeviceInfo { irlen: B( 6), name: "XC3S400AN",      specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   58955, slr: 1, family: Spartan3, 
        (id(0x02628093), DeviceInfo { irlen: B( 6), name: "XC3S700AN",      specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   85395, slr: 1, family: Spartan3, 
        (id(0x02630093), DeviceInfo { irlen: B( 6), name: "XC3S1400AN",     specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  148603, slr: 1, family: Spartan3, 
        (id(0x03840093), DeviceInfo { irlen: B( 6), name: "XC3SD1800",      specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  256165, slr: 1, family: Spartan3, 
        (id(0x0384E093), DeviceInfo { irlen: B( 6), name: "XC3SD3400",      specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  366197, slr: 1, family: Spartan3, 
        // Virtex-2
        (id(0x01008093), DeviceInfo { irlen: B( 6), name: "XC2V40",         specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   11253, slr: 1, family: Series2, 
        (id(0x01010093), DeviceInfo { irlen: B( 6), name: "XC2V80",         specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   19853, slr: 1, family: Series2, 
        (id(0x01018093), DeviceInfo { irlen: B( 6), name: "XC2V250",        specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   53037, slr: 1, family: Series2, 
        (id(0x01020093), DeviceInfo { irlen: B( 6), name: "XC2V500",        specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   86309, slr: 1, family: Series2, 
        (id(0x01028093), DeviceInfo { irlen: B( 6), name: "XC2V1000",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  127581, slr: 1, family: Series2, 
        (id(0x01030093), DeviceInfo { irlen: B( 6), name: "XC2V1500",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  176853, slr: 1, family: Series2, 
        (id(0x01038093), DeviceInfo { irlen: B( 6), name: "XC2V2000",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  234125, slr: 1, family: Series2, 
        (id(0x01040093), DeviceInfo { irlen: B( 6), name: "XC2V3000",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  327949, slr: 1, family: Series2, 
        (id(0x01050093), DeviceInfo { irlen: B( 6), name: "XC2V4000",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  489373, slr: 1, family: Series2, 
        (id(0x01060093), DeviceInfo { irlen: B( 6), name: "XC2V6000",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  682797, slr: 1, family: Series2, 
        (id(0x01070093), DeviceInfo { irlen: B( 6), name: "XC2V8000",       specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:  908221, slr: 1, family: Series2, 
        (id(0x00608093), DeviceInfo { irlen: B( 5), name: "XC2S15",         specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:    6178, slr: 1, family: Series2, 
        (id(0x0060C093), DeviceInfo { irlen: B( 5), name: "XC2S30",         specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   10524, slr: 1, family: Series2, 
        (id(0x00610093), DeviceInfo { irlen: B( 5), name: "XC2S50",         specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   17475, slr: 1, family: Series2, 
        (id(0x00614093), DeviceInfo { irlen: B( 5), name: "XC2S100",        specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   24413, slr: 1, family: Series2, 
        (id(0x00618093), DeviceInfo { irlen: B( 5), name: "XC2S150",        specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   32503, slr: 1, family: Series2, 
        (id(0x0061C093), DeviceInfo { irlen: B( 5), name: "XC2S200",        specific: S::Unknown }), // id_cmd: 0x09,  readback_bits:   41745, slr: 1, family: Series2, 
        // XC95XL
        (id(0x09602093), DeviceInfo { irlen: B( 8), name: "XC9536XL",       specific: S::Unknown }), // id_cmd: 0xfe,  readback_bits:       0, slr: 0, family: XC95XL,
        (id(0x09604093), DeviceInfo { irlen: B( 8), name: "XC9572XL",       specific: S::Unknown }), // id_cmd: 0xfe,  readback_bits:       0, slr: 0, family: XC95XL,
        (id(0x09608093), DeviceInfo { irlen: B( 8), name: "XC95144XL",      specific: S::Unknown }), // id_cmd: 0xfe,  readback_bits:       0, slr: 0, family: XC95XL,
        (id(0x09616093), DeviceInfo { irlen: B( 8), name: "XC95288XL",      specific: S::Unknown }), // id_cmd: 0xfe,  readback_bits:       0, slr:   0, fa mil y:XC95XL,
        // XC95XV
        (id(0x09702093), DeviceInfo { irlen: B( 8), name: "XC9536XV",       specific: S::Unknown }), // id_cmd: 0xfe,  readback_bits:       0, slr:   0, fa mil y:XC95XV,
        (id(0x09704093), DeviceInfo { irlen: B( 8), name: "XC9572XV",       specific: S::Unknown }), // id_cmd: 0xfe,  readback_bits:       0, slr:   0, fa mil y:XC95XV,
        (id(0x09708093), DeviceInfo { irlen: B( 8), name: "XC95144XV",      specific: S::Unknown }), // id_cmd: 0xfe,  readback_bits:       0, slr:   0, fa mil y:XC95XV,
        (id(0x09716093), DeviceInfo { irlen: B( 8), name: "XC95288XV",      specific: S::Unknown }), // id_cmd: 0xfe,  readback_bits:       0, slr:   0, fa mil y:XC95XV,
        // XC2C
        (id(0x06C1C093), DeviceInfo { irlen: B( 8), name: "XC2C32_VQ44",     specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06C1D093), DeviceInfo { irlen: B( 8), name: "XC2C32_PC44_64",  specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06C1B093), DeviceInfo { irlen: B( 8), name: "XC2C32A_QF32",    specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D1D093), DeviceInfo { irlen: B( 8), name: "XC2C32A_PC44",    specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06E1B093), DeviceInfo { irlen: B( 8), name: "XC2C32A_CP56",    specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06E1C093), DeviceInfo { irlen: B( 8), name: "XC2C32A_VQ44",    specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06E1D093), DeviceInfo { irlen: B( 8), name: "XC2C32A_PC44_64", specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06C5A093), DeviceInfo { irlen: B( 8), name: "XC2C64_PC44",     specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06C5B093), DeviceInfo { irlen: B( 8), name: "XC2C64_CP132",    specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06C5C093), DeviceInfo { irlen: B( 8), name: "XC2C64_VQ100",    specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06C5D093), DeviceInfo { irlen: B( 8), name: "XC2C64_CP56",     specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06C5E093), DeviceInfo { irlen: B( 8), name: "XC2C64_VQ44",     specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06E59093), DeviceInfo { irlen: B( 8), name: "XC2C64A_QF48",    specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06E5A093), DeviceInfo { irlen: B( 8), name: "XC2C64A_PC44",    specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06E5B093), DeviceInfo { irlen: B( 8), name: "XC2C64A_CP132",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06E5C093), DeviceInfo { irlen: B( 8), name: "XC2C64A_VQ100",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06E5D093), DeviceInfo { irlen: B( 8), name: "XC2C64A_CP56",    specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06E5E093), DeviceInfo { irlen: B( 8), name: "XC2C64A_VQ44",    specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D8A093), DeviceInfo { irlen: B( 8), name: "XC2C128_VQ100",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D8B093), DeviceInfo { irlen: B( 8), name: "XC2C128_CP132",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D8C093), DeviceInfo { irlen: B( 8), name: "XC2C128_TQ144",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D8E093), DeviceInfo { irlen: B( 8), name: "XC2C128_FT256",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D4A093), DeviceInfo { irlen: B( 8), name: "XC2C256_VQ100",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D4B093), DeviceInfo { irlen: B( 8), name: "XC2C256_CP132",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D4C093), DeviceInfo { irlen: B( 8), name: "XC2C256_TQ144",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D4D093), DeviceInfo { irlen: B( 8), name: "XC2C256_PQ208",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D4E093), DeviceInfo { irlen: B( 8), name: "XC2C256_FT256",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D5A093), DeviceInfo { irlen: B( 8), name: "XC2C384_FG324",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D5B093), DeviceInfo { irlen: B( 8), name: "XC2C384_CP204",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D5C093), DeviceInfo { irlen: B( 8), name: "XC2C384_TQ144",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D5D093), DeviceInfo { irlen: B( 8), name: "XC2C384_PQ208",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D5E093), DeviceInfo { irlen: B( 8), name: "XC2C384_FT256",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D7A093), DeviceInfo { irlen: B( 8), name: "XC2C512_FG324",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D7C093), DeviceInfo { irlen: B( 8), name: "XC2C512_PQ208",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        (id(0x06D7E093), DeviceInfo { irlen: B( 8), name: "XC2C512_FT256",   specific: S::Unknown }), // id_cmd: 0x01, readback_bits: 0, slr: 0, family: XC2C,
        // XC95
        (id(0x09502093), DeviceInfo { irlen: B( 8), name: "XC9536",          specific: S::Unknown }), // id_cmd: 0xfd, readback_bits: 0, slr: 0, family: XC95,
        (id(0x09504093), DeviceInfo { irlen: B( 8), name: "XC9572",          specific: S::Unknown }), // id_cmd: 0xfd, readback_bits: 0, slr: 0, family: XC95,
        (id(0x09506093), DeviceInfo { irlen: B( 8), name: "XC95108",         specific: S::Unknown }), // id_cmd: 0xfd, readback_bits: 0, slr: 0, family: XC95,
        (id(0x09508093), DeviceInfo { irlen: B( 8), name: "XC95144",         specific: S::Unknown }), // id_cmd: 0xfd, readback_bits: 0, slr: 0, family: XC95,
        (id(0x09512093), DeviceInfo { irlen: B( 8), name: "XC95216",         specific: S::Unknown }), // id_cmd: 0xfd, readback_bits: 0, slr: 0, family: XC95,
        (id(0x09516093), DeviceInfo { irlen: B( 8), name: "XC95288",         specific: S::Unknown }), // id_cmd: 0xfd, readback_bits: 0, slr: 0, family: XC95,
        // XC18
        (id(0x05024093), DeviceInfo { irlen: B( 8), name: "XC18V01",         specific: S::Unknown }), // id_cmd: 0xfd, readback_bits: 0, slr: 0, family: XC18,
        (id(0x05025093), DeviceInfo { irlen: B( 8), name: "XC18V02",         specific: S::Unknown }), // id_cmd: 0xfd, readback_bits: 0, slr: 0, family: XC18,
        (id(0x05026093), DeviceInfo { irlen: B( 8), name: "XC18V04",         specific: S::Unknown }), // id_cmd: 0xfd, readback_bits: 0, slr: 0, family: XC18,
        (id(0x05023093), DeviceInfo { irlen: B( 8), name: "XC18V512",        specific: S::Unknown }), // id_cmd: 0xfd, readback_bits: 0, slr: 0, family: XC18, 
        // ARM DAP on Zynq
        (id(0x0BA00477), DeviceInfo { irlen: B( 4), name: "ARM_DAP",         specific: S::Unknown }), // id_cmd: 0x09
    ];

    DEVICES.iter().cloned()
}
