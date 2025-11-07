use nafa_io::devices::Xilinx32Family as Family;

#[derive(Clone, Copy, Debug)]
pub struct Command {
    pub cmd: Cmd,
    pub addr: Addr,
    pub data: u16,
}

impl Command {
    pub fn to_bits(self) -> u32 {
        Self::to_bits_raw(self.cmd as _, self.addr as _, self.data)
    }

    pub fn to_bits_raw(cmd: u8, addr: u16, data: u16) -> u32 {
        (cmd as u32 & 0x0f) << 26 | (addr as u32 & 0x3ff) << 16 | (data as u32)
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum Cmd {
    Noop = 0b00,
    Read = 0b01,
    Write = 0b10,
}

/// The descriptions are taken from [UG480] (Series 7). However, the registers
/// are mostly the same for Ultrascale and Ultrascale+, detailed in [UG580].
///
/// [UG480], [Table 3-1]: Status Registers (Read Only)
///
/// [UG480]: https://docs.amd.com/api/khub/maps/qOeib0vlzXa1isUAfuFzOQ/attachments/_mT0t4XmsgJ2qfoNRTv53w-qOeib0vlzXa1isUAfuFzOQ/content
/// [UG580]: https://www.amd.com/content/dam/xilinx/support/documents/user_guides/ug580-ultrascale-sysmon.pdf
/// [Table 3-1]: https://docs.amd.com/api/khub/maps/qOeib0vlzXa1isUAfuFzOQ/attachments/_mT0t4XmsgJ2qfoNRTv53w-qOeib0vlzXa1isUAfuFzOQ/content#G6.307934
#[repr(u16)]
#[derive(Clone, Copy, Debug)]
pub enum Addr {
    /// The result of the on-chip temperature sensor measurement is stored in
    /// this location. The data is MSB justified in the 16-bit register. The 12
    /// MSBs correspond to the temperature sensor transfer function shown in
    /// [Figure 2-9].
    ///
    /// [Figure 2-9]: https://docs.amd.com/api/khub/maps/qOeib0vlzXa1isUAfuFzOQ/attachments/_mT0t4XmsgJ2qfoNRTv53w-qOeib0vlzXa1isUAfuFzOQ/content#M5.9.35661.XAnchorFigure.XRef.Target..Figure.18
    Temperature = 0x00,

    /// The result of the on-chip VCCINT supply monitor measurement is stored at
    /// this location. The data is MSB justified in the 16-bit register. The 12
    /// MSBs correspond to the supply sensor transfer function shown in [Figure
    /// 2-10].
    ///
    /// [Figure 2-10]: https://docs.amd.com/api/khub/maps/qOeib0vlzXa1isUAfuFzOQ/attachments/_mT0t4XmsgJ2qfoNRTv53w-qOeib0vlzXa1isUAfuFzOQ/content#M5.9.17020.XAnchorFigure.XRef.Target..Figure.19
    VccInt = 0x01,

    /// The result of the on-chip VCCAUX data supply monitor measurement is
    /// stored at this location. The data is MSB justified in the 16 bit
    /// register. The 12 MSBs correspond to the supply sensor transfer function
    /// shown in [Figure 2-10].
    ///
    /// [Figure 2-10]: https://docs.amd.com/api/khub/maps/qOeib0vlzXa1isUAfuFzOQ/attachments/_mT0t4XmsgJ2qfoNRTv53w-qOeib0vlzXa1isUAfuFzOQ/content#M5.9.17020.XAnchorFigure.XRef.Target..Figure.19
    VccAux = 0x02,

    /// The result of a conversion on the dedicated analog input channel is
    /// stored in this register. The data is MSB justified in the 16-bit
    /// register. The 12 MSBs correspond to the transfer function shown in
    /// [Figure 2-6], or [Figure 2-7], depending on analog input mode settings.
    ///
    /// [Figure 2-6]: https://docs.amd.com/api/khub/maps/qOeib0vlzXa1isUAfuFzOQ/attachments/_mT0t4XmsgJ2qfoNRTv53w-qOeib0vlzXa1isUAfuFzOQ/content#M5.9.52189.XAnchorFigure.XRef.Target..Figure.15
    /// [Figure 2-7]: https://docs.amd.com/api/khub/maps/qOeib0vlzXa1isUAfuFzOQ/attachments/_mT0t4XmsgJ2qfoNRTv53w-qOeib0vlzXa1isUAfuFzOQ/content#M5.9.12554.XAnchorFigure.XRef.Target..Figure.16
    VpVn = 0x03,

    /// The result of a conversion on the reference input VREFP is stored in
    /// this register. The 12 MSBs correspond to the ADC transfer function shown
    /// in [Figure 2-10]. The data is MSB justified in the 16-bit register. The
    /// supply sensor is used when measuring V REFP .
    ///
    /// [Figure 2-10]: https://docs.amd.com/api/khub/maps/qOeib0vlzXa1isUAfuFzOQ/attachments/_mT0t4XmsgJ2qfoNRTv53w-qOeib0vlzXa1isUAfuFzOQ/content#M5.9.17020.XAnchorFigure.XRef.Target..Figure.19
    VRefP = 0x04,

    /// The result of a conversion on the reference input VREFN is stored in
    /// this register. This channel is measured in bipolar mode with a two's
    /// complement output coding as shown in [Figure 2-3]. By measuring in
    /// bipolar mode, small positive and negative offset around 0V (VREFN ) can
    /// be measured. The supply sensor is also used to measure VREFN , thus 1
    /// LSB = 3V/4096. The data is MSB justified in the 16-bit register.
    ///
    /// [Figure 2-3]: https://docs.amd.com/api/khub/maps/qOeib0vlzXa1isUAfuFzOQ/attachments/_mT0t4XmsgJ2qfoNRTv53w-qOeib0vlzXa1isUAfuFzOQ/content#M5.9.78910.XAnchorFigure.XRef.Target..Figure.12
    VRefN = 0x05,

    /// The result of the on-chip VCCBRAM supply monitor measurement is stored
    /// at this location. The data is MSB justified in the 16-bit register. The
    /// 12 MSBs correspond to the supply sensor transfer function shown in
    /// [Figure 2-10].
    ///
    /// [Figure 2-10]: https://docs.amd.com/api/khub/maps/qOeib0vlzXa1isUAfuFzOQ/attachments/_mT0t4XmsgJ2qfoNRTv53w-qOeib0vlzXa1isUAfuFzOQ/content#M5.9.17020.XAnchorFigure.XRef.Target..Figure.19
    VccBram = 0x06,

    /// The calibration coefficient for the supply sensor offset using ADC A is
    /// stored at this location.
    SupplyAOffset = 0x08,

    /// The calibration coefficient for the ADC A offset is stored at this
    /// location.
    AdcAOffset = 0x09,

    /// The calibration coefficient for the ADC A gain error is stored at this
    /// location.
    AdcAGain = 0x0a,

    /// The result of a conversion on the PS supply, VCCPINT is stored in this
    /// register. The 12 MSBs correspond to the ADC transfer function shown in
    /// [Figure 2-10]. The data is MSB justified in the 16-bit register. The
    /// supply sensor is used when measuring VCCPINT.
    ///
    /// [Figure 2-10]: https://docs.amd.com/api/khub/maps/qOeib0vlzXa1isUAfuFzOQ/attachments/_mT0t4XmsgJ2qfoNRTv53w-qOeib0vlzXa1isUAfuFzOQ/content#M5.9.17020.XAnchorFigure.XRef.Target..Figure.19
    VccPInt = 0x0d,

    /// The result of a conversion on the PS supply, VCCPAUX is stored in this
    /// register. The 12 MSBs correspond to the ADC transfer function shown in
    /// [Figure 2-10]. The data is MSB justified in the 16-bit register. The
    /// supply sensor is used when measuring VCCPAUX.
    ///
    /// [Figure 2-10]: https://docs.amd.com/api/khub/maps/qOeib0vlzXa1isUAfuFzOQ/attachments/_mT0t4XmsgJ2qfoNRTv53w-qOeib0vlzXa1isUAfuFzOQ/content#M5.9.17020.XAnchorFigure.XRef.Target..Figure.19
    VccPAux = 0x0e,

    /// The result of a conversion on the PS supply, VCCO_DDR is stored in this
    /// register. The 12 MSBs correspond to the ADC transfer function shown in
    /// [Figure 2-10]. The data is MSB justified in the 16-bit register. The
    /// supply sensor is used when measuring VCCO_DDR.
    ///
    /// [Figure 2-10]: https://docs.amd.com/api/khub/maps/qOeib0vlzXa1isUAfuFzOQ/attachments/_mT0t4XmsgJ2qfoNRTv53w-qOeib0vlzXa1isUAfuFzOQ/content#M5.9.17020.XAnchorFigure.XRef.Target..Figure.19
    VccODdr = 0x0f,

    // TODO: unsure if this is the correct interpretation of these registers
    /// The results of the conversions on auxiliary analog input channels are
    /// stored in this register. The data is MSB justified in the 16-bit
    /// register. The 12 MSBs correspond to the transfer function shown in
    /// [Figure 2-2] or [Figure 2-3] depending on analog input mode settings.
    ///
    /// [Figure 2-2]: https://docs.amd.com/api/khub/maps/qOeib0vlzXa1isUAfuFzOQ/attachments/_mT0t4XmsgJ2qfoNRTv53w-qOeib0vlzXa1isUAfuFzOQ/content#M5.9.73167.XAnchorFigure.XRef.Target..Figure.11
    /// [Figure 2-3]: https://docs.amd.com/api/khub/maps/qOeib0vlzXa1isUAfuFzOQ/attachments/_mT0t4XmsgJ2qfoNRTv53w-qOeib0vlzXa1isUAfuFzOQ/content#M5.9.78910.XAnchorFigure.XRef.Target..Figure.12
    VAuxPVAuxN0 = 0x10,
    VAuxPVAuxN1 = 0x11,
    VAuxPVAuxN2 = 0x12,
    VAuxPVAuxN3 = 0x13,
    VAuxPVAuxN4 = 0x14,
    VAuxPVAuxN5 = 0x15,
    VAuxPVAuxN6 = 0x16,
    VAuxPVAuxN7 = 0x17,
    VAuxPVAuxN8 = 0x18,
    VAuxPVAuxN9 = 0x19,
    VAuxPVAuxNA = 0x1a,
    VAuxPVAuxNB = 0x1b,
    VAuxPVAuxNC = 0x1c,
    VAuxPVAuxND = 0x1d,
    VAuxPVAuxNE = 0x1e,
    VAuxPVAuxNF = 0x1f,

    /// Maximum temperature measurement recorded since power-up or the last XADC
    /// reset.
    MaxTemp = 0x20,

    /// Maximum VCCINT measurement recorded since power-up or the last XADC
    /// reset.
    MaxVccInt = 0x21,

    /// Maximum VCCAUX measurement recorded since power-up or the last XADC
    /// reset.
    MaxVccAux = 0x22,

    /// Maximum VCCBRAM measurement recorded since power-up or the last XADC
    /// reset.
    MaxVccBram = 0x23,

    /// Minimum temperature measurement recorded since power-up or the last XADC
    /// reset.
    MinTemp = 0x24,

    /// Minimum V CCINT measurement recorded since power-up or the last XADC
    /// reset.
    MinVccInt = 0x25,

    /// Minimum V CCAUX measurement recorded since power-up or the last XADC
    /// reset.
    MinVccAux = 0x26,

    /// Minimum V CCBRAM measurement recorded since power-up or the last XADC
    /// reset.
    MinVccBram = 0x27,

    /// Maximum VCCPINT measurement recorded since power-up or the last XADC
    /// reset.
    VccPIntMax = 0x28,

    /// Maximum VCCPAUX measurement recorded since power-up or the last XADC
    /// reset.
    VccPAuxMax = 0x29,

    /// Maximum VCCO_DDR measurement recorded since power-up or the last XADC
    /// reset.
    VccODdrMax = 0x2a,

    /// Minimum V CCPINT measurement recorded since power-up or the last XADC
    /// reset.
    VccPIntMin = 0x2c,

    /// Minimum V CCAUX measurement recorded since power-up or the last XADC
    /// reset.
    VccPAuxMin = 0x2d,

    /// Minimum V CCO_DDR measurement recorded since power-up or the last XADC
    /// reset.
    VccODdrMin = 0x2e,

    /// The calibration coefficient for the supply sensor offset using ADC B is
    /// stored at this location.
    SupplyBOffset = 0x30,

    /// The calibration coefficient for the ADC B offset is stored at this
    /// location.
    AdcBOffset = 0x31,

    /// The calibration coefficient for the ADC B gain error is stored at this
    /// location.
    AdcBGain = 0x32,

    /// This register contains general status information (see [Flag Register]).
    ///
    /// [Flag Register]: https://docs.amd.com/api/khub/maps/qOeib0vlzXa1isUAfuFzOQ/attachments/_mT0t4XmsgJ2qfoNRTv53w-qOeib0vlzXa1isUAfuFzOQ/content#G6.301009
    Flag = 0x3f,
}

pub enum Transfer {
    None,
    Unknown,
    Exactly(fn(u16) -> f32),
    OneOf(&'static [fn(u16) -> f32]),
}

impl Addr {
    pub fn transfer(self, family: Family) -> Transfer {
        match self {
            Addr::Temperature | Addr::MaxTemp | Addr::MinTemp => temperature(family),

            Addr::VccInt
            | Addr::VccAux
            | Addr::VRefP
            | Addr::VRefN
            | Addr::VccBram
            | Addr::VccPInt
            | Addr::VccPAux
            | Addr::VccODdr
            | Addr::MaxVccInt
            | Addr::MaxVccAux
            | Addr::MinVccInt
            | Addr::MinVccAux
            | Addr::MinVccBram
            | Addr::VccPIntMax
            | Addr::VccPAuxMax
            | Addr::VccODdrMax
            | Addr::VccPIntMin
            | Addr::VccPAuxMin
            | Addr::VccODdrMin => power_supply(family),

            Addr::VpVn
            | Addr::VAuxPVAuxN0
            | Addr::VAuxPVAuxN1
            | Addr::VAuxPVAuxN2
            | Addr::VAuxPVAuxN3
            | Addr::VAuxPVAuxN4
            | Addr::VAuxPVAuxN5
            | Addr::VAuxPVAuxN6
            | Addr::VAuxPVAuxN7
            | Addr::VAuxPVAuxN8
            | Addr::VAuxPVAuxN9
            | Addr::VAuxPVAuxNA
            | Addr::VAuxPVAuxNB
            | Addr::VAuxPVAuxNC
            | Addr::VAuxPVAuxND
            | Addr::VAuxPVAuxNE
            | Addr::VAuxPVAuxNF => Transfer::OneOf(&[adc_unipolar_s7, adc_bipolar_s7]),

            _ => Transfer::None,
        }
    }
}

pub fn temperature(family: Family) -> Transfer {
    const _2_10: f32 = (2 << (10 - 1)) as f32;
    match family {
        Family::S7 => Transfer::Exactly(temperature_s7),
        Family::US => Transfer::OneOf(&[
            |d| linear_scale_10(d, -273.8195, 502.9098 / _2_10), // sysmone1, external ref
            |d| linear_scale_10(d, -273.6777, 501.3743 / _2_10), // sysmone1, internal ref
        ]),
        Family::UP => Transfer::OneOf(&[
            |d| linear_scale_10(d, -273.8195, 502.9098 / _2_10), // sysmone1, external ref
            |d| linear_scale_10(d, -273.6777, 501.3743 / _2_10), // sysmone1, internal ref
            |d| linear_scale_10(d, -279.4266, 507.5921 / _2_10), // sysmone4, external ref
            |d| linear_scale_10(d, -280.2309, 509.3141 / _2_10), // sysmone4, internal ref
        ]),
        Family::Z7 | Family::ZP | Family::Versal => Transfer::Unknown,
    }
}

pub fn power_supply(family: Family) -> Transfer {
    match family {
        Family::S7 => Transfer::Exactly(power_supply_s7),
        Family::US | Family::UP => Transfer::Exactly(power_supply_us),
        Family::Z7 | Family::ZP | Family::Versal => Transfer::Unknown,
    }
}

pub fn adc(family: Family) -> Transfer {
    match family {
        Family::S7 => Transfer::OneOf(&[adc_unipolar_s7, adc_bipolar_s7]),
        Family::US | Family::UP => Transfer::OneOf(&[adc_unipolar_us, adc_bipolar_us]),
        Family::Z7 | Family::ZP | Family::Versal => Transfer::Unknown,
    }
}

pub fn power_supply_us(data: u16) -> f32 {
    linear_scale_10(data, 0., 0.00293)
}

pub fn temperature_s7(data: u16) -> f32 {
    linear_scale_12(data, -273., 0.123)
}

pub fn power_supply_s7(data: u16) -> f32 {
    linear_scale_12(data, 0., 0.000732)
}

pub fn adc_unipolar_us(data: u16) -> f32 {
    linear_scale_10(data, 0., 1. / 1024.)
}

pub fn adc_bipolar_us(data: u16) -> f32 {
    linear_scale_10_signed(data, 0., 1. / 1024.)
}

pub fn adc_unipolar_s7(data: u16) -> f32 {
    linear_scale_12(data, 0., 1. / 4096.)
}

pub fn adc_bipolar_s7(data: u16) -> f32 {
    linear_scale_12_signed(data, 0., 1. / 4096.)
}

fn linear_scale_10(data: u16, base: f32, step: f32) -> f32 {
    let val = (data >> 6) as f32;
    val.mul_add(step, base)
}

fn linear_scale_10_signed(data: u16, base: f32, step: f32) -> f32 {
    let val = (data as i16 >> 6) as f32;
    val.mul_add(step, base)
}

fn linear_scale_12(data: u16, base: f32, step: f32) -> f32 {
    let val = (data >> 4) as f32;
    val.mul_add(step, base)
}

fn linear_scale_12_signed(data: u16, base: f32, step: f32) -> f32 {
    let val = (data as i16 >> 4) as f32;
    val.mul_add(step, base)
}
