
#[derive(Default)]
struct Fuses {
    // efuse registers
    user1: [u8; 4],
    user2: [u8; 4],

    usercode: [u8; 4],
}

// Status Register struct of bitstream registers to be used within each of the family structs
#[derive(Default)]
struct StatReg {
    config_sts: [u8; 4],
    cor1: [u8; 2],
    co2: [u8; 2],
    ctl: [u8; 4],
    pwrdwn: [u8; 2],
    hc_opt: [u8; 2],
}

// Spartan 3s
#[derive(Default)]
pub struct S3 {
    // efuse registers
    fuse: Fuses,
    user3: [u8; 4],
    user4: [u8; 4],
    cntl: [u8; 4],
    dna: [u8; 4], // must be preceded by ISC_ENABLE and followed by ISC_DISABLE

    // bitstream registers
    status: StatReg,
}

// 6 Series and 6 Series EX devices
#[derive(Default)]
pub struct S6 {
    // efuse registers
    fuse: Fuses,
    user3: [u8; 4],
    user4: [u8; 4],
    cntl: [u8; 4],
    dna: [u8; 4], // must be preceded by ISC_ENABLE and followed by ISC_DISABLE

    // bitstream registers
    status: StatReg,
}
