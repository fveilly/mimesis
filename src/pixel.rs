use std::num::ParseIntError;
use std::ops::{Add, AddAssign, BitAnd, BitOr, BitXor, Deref, Div, DivAssign, Mul, MulAssign, Not, Rem, Sub, SubAssign};
use image::{Pixel, Primitive};
use num_traits::{Bounded, Num, NumCast, One, ToPrimitive, Zero};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bit(pub bool);

impl Deref for Bit {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<bool> for Bit {
    fn from(value: bool) -> Self {
        Bit(value)
    }
}

impl AddAssign for Bit {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl SubAssign for Bit {
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl MulAssign for Bit {
    fn mul_assign(&mut self, rhs: Self) {
        *self = *self * rhs;
    }
}

impl DivAssign for Bit {
    fn div_assign(&mut self, rhs: Self) {
        *self = *self / rhs;
    }
}

impl Add for Bit {
    type Output = Self;
    fn add(mut self, rhs: Self) -> Self::Output {
        self.0 |= *rhs;
        self
    }
}

impl Sub for Bit {
    type Output = Self;
    fn sub(mut self, rhs: Self) -> Self::Output {
        self.0 ^= *rhs;
        self
    }
}

impl Mul for Bit {
    type Output = Self;
    fn mul(mut self, rhs: Self) -> Self::Output {
        self.0 &= *rhs;
        self
    }
}

impl Div for Bit {
    type Output = Self;
    fn div(mut self, rhs: Self) -> Self::Output {
        debug_assert!(*rhs, "Cannot divide by zero");
        self.0 &= *rhs;
        self
    }
}

impl Rem for Bit {
    type Output = Self;
    fn rem(self, rhs: Self) -> Self::Output {
        debug_assert!(*rhs, "Cannot divide by zero");
        Self(false)
    }
}

impl BitOr for Bit {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Bit(self.0 | rhs.0)
    }
}

impl BitAnd for Bit {
    type Output = Self;
    fn bitand(self, rhs: Self) -> Self::Output {
        Bit(self.0 & rhs.0)
    }
}

impl BitXor for Bit {
    type Output = Self;
    fn bitxor(self, rhs: Self) -> Self::Output {
        Bit(self.0 ^ rhs.0)
    }
}

impl Not for Bit {
    type Output = Self;
    fn not(self) -> Self::Output {
        Bit(!self.0)
    }
}

impl Zero for Bit {
    fn zero() -> Self {
        Self(false)
    }
    fn set_zero(&mut self) {
        self.0 = false;
    }
    fn is_zero(&self) -> bool {
        !**self
    }
}

impl One for Bit {
    fn one() -> Self {
        Self(true)
    }
    fn set_one(&mut self) {
        self.0 = true;
    }
    fn is_one(&self) -> bool {
        **self
    }
}

impl Pixel for Bit {
    type Subpixel = Self;
    const CHANNEL_COUNT: u8 = 1;
    fn channels(&self) -> &[Self] {
        unimplemented!()
    }
    fn channels_mut(&mut self) -> &mut [Self] {
        unimplemented!()
    }
    const COLOR_MODEL: &'static str = "BLACKANDWHITE";
    fn channels4(&self) -> (Self, Self, Self, Self) {
        (*self, *self, *self, *self)
    }
    fn from_channels(a: Self, b: Self, c: Self, d: Self) -> Self {
        Self(*(a | b | c | d))
    }
    fn from_slice(slice: &[Self]) -> &Self {
        assert_eq!(slice.len(), 1);
        unsafe { &*slice.as_ptr().cast() }
    }
    fn from_slice_mut(slice: &mut [Self]) -> &mut Self {
        assert_eq!(slice.len(), 1);
        unsafe { &mut *slice.as_mut_ptr().cast() }
    }
    fn to_rgb(&self) -> image::Rgb<Bit> {
        image::Rgb([if **self { One::one() } else { Zero::zero() }; 3])
    }
    fn to_rgba(&self) -> image::Rgba<Bit> {
        image::Rgba([if **self { One::one() } else { Zero::zero() }; 4])
    }
    fn to_luma(&self) -> image::Luma<Self> {
        image::Luma([if **self { One::one() } else { Zero::zero() }])
    }
    fn to_luma_alpha(&self) -> image::LumaA<Self> {
        image::LumaA([if **self { One::one() } else { Zero::zero() }; 2])
    }
    fn map<F>(&self, mut f: F) -> Self
    where
        F: FnMut(Self) -> Self,
    {
        Self(*f(*self))
    }
    fn apply<F>(&mut self, mut f: F)
    where
        F: FnMut(Self) -> Self,
    {
        self.0 = *f(*self);
    }
    fn map_with_alpha<F, G>(&self, mut f: F, _: G) -> Self
    where
        F: FnMut(Self) -> Self,
        G: FnMut(Self) -> Self,
    {
        Self(*f(*self))
    }
    fn apply_with_alpha<F, G>(&mut self, f: F, _: G)
    where
        F: FnMut(Self) -> Self,
        G: FnMut(Self) -> Self,
    {
        self.apply(f);
    }
    fn map_without_alpha<F>(&self, mut f: F) -> Self
    where
        F: FnMut(Self) -> Self,
    {
        Self(*f(*self))
    }
    fn apply_without_alpha<F>(&mut self, f: F)
    where
        F: FnMut(Self) -> Self,
    {
        self.apply(f);
    }
    fn map2<F>(&self, other: &Self, mut f: F) -> Self
    where
        F: FnMut(Self, Self) -> Self,
    {
        Self(*f(*self, *other))
    }
    fn apply2<F>(&mut self, other: &Self, mut f: F)
    where
        F: FnMut(Self, Self) -> Self,
    {
        self.0 = *f(*self, *other);
    }
    fn invert(&mut self) {
        self.0 = !self.0;
    }
    fn blend(&mut self, other: &Self) {
        *self -= *other;
    }
}


impl Bounded for Bit {
    fn min_value() -> Self {
        Bit(false)
    }

    fn max_value() -> Self {
        Bit(true)
    }
}

impl Num for Bit {
    type FromStrRadixErr = ParseIntError;

    fn from_str_radix(s: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        let n = u32::from_str_radix(s, radix)?;
        Ok(Bit(n != 0))
    }
}

impl ToPrimitive for Bit {
    fn to_isize(&self) -> Option<isize> {
        Some(self.0 as isize)
    }

    fn to_i8(&self) -> Option<i8> {
        Some(self.0 as i8)
    }

    fn to_i16(&self) -> Option<i16> {
        Some(self.0 as i16)
    }

    fn to_i32(&self) -> Option<i32> {
        Some(self.0 as i32)
    }

    fn to_i64(&self) -> Option<i64> {
        Some(self.0 as i64)
    }

    fn to_usize(&self) -> Option<usize> {
        Some(self.0 as usize)
    }

    fn to_u8(&self) -> Option<u8> {
        Some(self.0 as u8)
    }

    fn to_u16(&self) -> Option<u16> {
        Some(self.0 as u16)
    }

    fn to_u32(&self) -> Option<u32> {
        Some(self.0 as u32)
    }

    fn to_u64(&self) -> Option<u64> {
        Some(self.0 as u64)
    }

    fn to_f32(&self) -> Option<f32> {
        Some(self.0 as u8 as f32)
    }

    fn to_f64(&self) -> Option<f64> {
        Some(self.0 as u8 as f64)
    }
}

impl NumCast for Bit {
    fn from<T: ToPrimitive>(n: T) -> Option<Self> {
        Some(Bit(n.to_u8()? != 0))
    }
}

impl Primitive for Bit {
    const DEFAULT_MAX_VALUE: Self = Self(true);
    const DEFAULT_MIN_VALUE: Self = Self(false);
}

impl<T: Primitive> From<image::Rgb<T>> for Bit {
    fn from(pixel: image::Rgb<T>) -> Self {
        Self(!pixel.0.iter().all(Zero::is_zero))
    }
}
impl<T: Primitive> From<image::Luma<T>> for Bit {
    fn from(pixel: image::Luma<T>) -> Self {
        Self(!pixel.0[0].is_zero())
    }
}
impl<T: Primitive> From<image::Rgba<T>> for Bit {
    fn from(pixel: image::Rgba<T>) -> Self {
        Self(!pixel.0[3].is_zero())
    }
}
impl<T: Primitive> From<image::LumaA<T>> for Bit {
    fn from(pixel: image::LumaA<T>) -> Self {
        Self(!pixel.0[1].is_zero())
    }
}

impl<T: Primitive> From<&image::Rgb<T>> for Bit {
    fn from(pixel: &image::Rgb<T>) -> Self {
        Self(!pixel.0.iter().all(Zero::is_zero))
    }
}
impl<T: Primitive> From<&image::Luma<T>> for Bit {
    fn from(pixel: &image::Luma<T>) -> Self {
        Self(!pixel.0[0].is_zero())
    }
}
impl<T: Primitive> From<&image::Rgba<T>> for Bit {
    fn from(pixel: &image::Rgba<T>) -> Self {
        Self(!pixel.0[3].is_zero())
    }
}
impl<T: Primitive> From<&image::LumaA<T>> for Bit {
    fn from(pixel: &image::LumaA<T>) -> Self {
        Self(!pixel.0[1].is_zero())
    }
}