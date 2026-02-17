use std::ops::{Div, Mul, Rem};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bits<T>(pub T);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bytes<T>(pub T);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

// note: `.into()` can't use the trait because there's a
// ```
// impl From<T> for T { ... }
// ```
// in the standard library

impl<T> Bits<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Bits<U> {
        Bits(f(self.0))
    }
    pub fn into_<U: From<T>>(self) -> Bits<U> {
        Bits(self.0.into())
    }
}
impl<T> Bytes<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Bytes<U> {
        Bytes(f(self.0))
    }
    pub fn into_<U: From<T>>(self) -> Bytes<U> {
        Bytes(self.0.into())
    }
}
impl<T> Words32<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Words32<U> {
        Words32(f(self.0))
    }
    pub fn into_<U: From<T>>(self) -> Words32<U> {
        Words32(self.0.into())
    }
}
