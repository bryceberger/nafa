use std::ops::{Div, Mul, Rem};

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct Bits<T>(pub T);

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct Bytes<T>(pub T);

#[repr(transparent)]
pub struct Words32<T>(pub T);

impl<T> From<Bytes<T>> for Bits<T>
where
    T: Mul<usize, Output = T>,
{
    fn from(value: Bytes<T>) -> Self {
        Self(value.0 * 8)
    }
}

impl<T> From<Words32<T>> for Bytes<T>
where
    T: Mul<usize, Output = T>,
{
    fn from(value: Words32<T>) -> Self {
        Self(value.0 * 4)
    }
}

impl<T> Bits<T>
where
    T: Copy,
    T: Div<usize, Output = T>,
    T: Rem<usize, Output = T>,
{
    pub fn as_pair(self) -> (Bytes<T>, Bits<T>) {
        (Bytes(self.0 / 8), Bits(self.0 % 8))
    }
}
