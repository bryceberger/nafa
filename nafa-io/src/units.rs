#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct Bits<T>(pub T);

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct Bytes<T>(pub T);

impl<T> From<Bytes<T>> for Bits<T>
where
    T: std::ops::Mul<usize, Output = T>,
{
    fn from(value: Bytes<T>) -> Self {
        Self(value.0 * 8)
    }
}

impl<T> Bits<T>
where
    T: Copy,
    T: std::ops::Div<usize, Output = T>,
    T: std::ops::Rem<usize, Output = T>,
{
    pub fn as_pair(self) -> (Bytes<T>, Bits<T>) {
        (Bytes(self.0 / 8), Bits(self.0 % 8))
    }
}
