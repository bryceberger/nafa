use std::fmt::Display;

pub struct Hex<T>(pub T);
impl Display for Hex<u8> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:02X}", self.0)
    }
}
impl Display for Hex<u16> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:04X}", self.0)
    }
}

const MAX_DISPLAY: usize = 16;

pub struct ShortHex<'a>(pub &'a [u8]);
impl Display for ShortHex<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for e in self.0.iter().take(MAX_DISPLAY) {
            write!(f, "{:02X}", e)?;
        }
        if self.0.len() > MAX_DISPLAY {
            write!(f, "...")?;
        }
        Ok(())
    }
}

pub struct SpaceHex<'a>(pub &'a [u8]);
impl Display for SpaceHex<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for e in self.0.iter().take(MAX_DISPLAY) {
            write!(f, "{:02X} ", e)?;
        }
        if self.0.len() > MAX_DISPLAY {
            write!(f, "...")?;
        }
        Ok(())
    }
}
