#[derive(Clone, Copy)]
pub struct Crc {
    data: u32,
}

impl Crc {
    pub fn new(init: u32) -> Self {
        Self { data: init }
    }

    pub fn update(&mut self, reg: u8, data: u32) {
        fn calc(mut current: u32, mut data: u32, bits: u32) -> u32 {
            const POLY: u32 = 0x82F6_3B78;
            for _ in 0..bits {
                let xor = current & 1 != data & 1;
                current >>= 1;
                data >>= 1;
                if xor {
                    current ^= POLY;
                }
            }
            current
        }
        self.data = calc(self.data, data, 32);
        self.data = calc(self.data, reg.into(), 5);
    }

    pub fn value(self) -> u32 {
        self.data
    }
}
