#[rustfmt::skip]
mod consts {
    #![allow(dead_code)]
    use super::Info;

    pub const BBV2:         Info = Info { dbus_data: 0x00, dbus_en: 0x10, cbus_data: 0x00, cbus_en: 0x00, };
    pub const DLP2232H:     Info = Info { dbus_data: 0x00, dbus_en: 0x10, cbus_data: 0x00, cbus_en: 0x00, };
    pub const IKDA:         Info = Info { dbus_data: 0x00, dbus_en: 0x00, cbus_data: 0x00, cbus_en: 0x04, };
    pub const LLBBC:        Info = Info { dbus_data: 0x00, dbus_en: 0x00, cbus_data: 0x00, cbus_en: 0x04, };
    pub const LLIF:         Info = Info { dbus_data: 0x10, dbus_en: 0x10, cbus_data: 0x00, cbus_en: 0x00, };
    pub const LLBUS:        Info = Info { dbus_data: 0x00, dbus_en: 0x10, cbus_data: 0x00, cbus_en: 0x00, };
    pub const FTDIJTAG:     Info = Info { dbus_data: 0x00, dbus_en: 0x10, cbus_data: 0x00, cbus_en: 0x00, };
    pub const FT2232TEST:   Info = Info { dbus_data: 0x00, dbus_en: 0x10, cbus_data: 0x00, cbus_en: 0x80, };
    pub const L_MOTCTL:     Info = Info { dbus_data: 0x00, dbus_en: 0x00, cbus_data: 0x00, cbus_en: 0x40, };
    pub const L_MOTCTL_AVR: Info = Info { dbus_data: 0x00, dbus_en: 0x00, cbus_data: 0x00, cbus_en: 0x00, };
    pub const KNOB2USB:     Info = Info { dbus_data: 0x00, dbus_en: 0x10, cbus_data: 0x00, cbus_en: 0x40, };
    pub const QM07_PU:      Info = Info { dbus_data: 0x00, dbus_en: 0x10, cbus_data: 0x00, cbus_en: 0x04, };
    pub const NEXSYS4:      Info = Info { dbus_data: 0xe8, dbus_en: 0xeb, cbus_data: 0x00, cbus_en: 0x60, };
}
pub use consts::*;

#[derive(Debug)]
pub struct Info {
    pub dbus_data: u8,
    pub dbus_en: u8,
    pub cbus_data: u8,
    pub cbus_en: u8,
}
