#[rustfmt::skip]
mod consts {
    use super::Info;
    use super::Interface::*;

    pub const AMONTEC:       Info = Info::new(B, 0x00, 0x10, 0x00, 0x00);
    pub const ARM_USB_OCD_H: Info = Info::new(B, 0x00, 0x10, 0x00, 0x08);
    pub const BBV2:          Info = Info::new(B, 0x00, 0x10, 0x00, 0x00);
    pub const BBV2_2:        Info = Info::new(C, 0x00, 0x00, 0x00, 0x00);
    pub const CM1:           Info = Info::new(A, 0x00, 0x00, 0x00, 0x00);
    pub const DLP2232H:      Info = Info::new(B, 0x00, 0x10, 0x00, 0x00);
    pub const FT2232TEST:    Info = Info::new(B, 0x00, 0x10, 0x00, 0x80);
    pub const FT4232H:       Info = Info::new(A, 0x00, 0x00, 0x00, 0x00);
    pub const FTDIJTAG:      Info = Info::new(B, 0x00, 0x10, 0x00, 0x00);
    pub const IKDA:          Info = Info::new(B, 0x00, 0x00, 0x00, 0x04);
    pub const JTAGHS2:       Info = Info::new(A, 0xe8, 0xeb, 0x00, 0x60);
    pub const KNOB2USB:      Info = Info::new(A, 0x00, 0x10, 0x00, 0x40);
    pub const L_MOTCTL:      Info = Info::new(B, 0x00, 0x00, 0x00, 0x40);
    pub const LLBBC:         Info = Info::new(C, 0x00, 0x00, 0x00, 0x04);
    pub const LLBUS:         Info = Info::new(C, 0x00, 0x10, 0x00, 0x00);
    pub const LLIF:          Info = Info::new(C, 0x10, 0x10, 0x00, 0x00);
    pub const MIMAS_A7:      Info = Info::new(C, 0x00, 0x4B, 0x00, 0x00);
    pub const NEXYS4:        Info = Info::new(A, 0xe8, 0xeb, 0x00, 0x60);
    pub const OLIMEX:        Info = Info::new(B, 0x00, 0x10, 0x00, 0x08);
    pub const PLUGJTAG:      Info = Info::new(B, 0x00, 0x10, 0x00, 0x00);
    pub const QM07_PU:       Info = Info::new(A, 0x00, 0x10, 0x00, 0x04);
    pub const TUMPA:         Info = Info::new(B, 0x00, 0x00, 0x00, 0x00);
    pub const TURTELIZER:    Info = Info::new(A, 0x00, 0x10, 0x00, 0x00);

}
pub use consts::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub(crate) enum Interface {
    A = 0,
    B = 1,
    C = 2,
    #[expect(unused)]
    D = 3,
}

#[derive(Debug)]
pub struct Info {
    pub(super) interface: Interface,
    pub(super) dbus_data: u8,
    pub(super) dbus_en: u8,
    pub(super) cbus_data: u8,
    pub(super) cbus_en: u8,
}

impl Info {
    const fn new(
        interface: Interface,
        dbus_data: u8,
        dbus_en: u8,
        cbus_data: u8,
        cbus_en: u8,
    ) -> Self {
        Self {
            interface,
            dbus_data,
            dbus_en,
            cbus_data,
            cbus_en,
        }
    }
}
