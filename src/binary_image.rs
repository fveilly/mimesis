use std::ops::Deref;
use bit_vec::BitVec;
use image::{GenericImage, GenericImageView, GrayImage, Pixel};
use num_traits::{ToPrimitive, Zero};
use crate::pixel::Bit;

#[derive(Debug, Clone, Default)]
pub struct BinaryImage {
    width: u32,
    height: u32,
    buffer: BitVec,
}

impl BinaryImage {
    #[inline]
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            buffer: BitVec::with_capacity((width * height) as usize),
        }
    }

    #[must_use]
    pub fn from_raw<T>(width: u32, height: u32, buffer: &[T]) -> Self
    where
        T: Zero,
    {
        let image_size = (width * height) as usize;
        debug_assert!(
            buffer.len() >= image_size,
            "Buffer must not be smaller than image dimensions"
        );
        let compress_step = buffer.len() / image_size;
        Self {
            buffer: buffer
                .chunks(compress_step)
                .map(|pixel| !pixel.iter().any(Zero::is_zero))
                .collect(),
            height,
            width,
        }
    }

    #[must_use]
    pub fn from_bitvec(width: u32, height: u32, buffer: BitVec) -> Self {
        let image_size = (width * height) as usize;
        debug_assert!(
            buffer.len() >= image_size,
            "Buffer must not be smaller than image dimensions"
        );
        Self {
            width,
            height,
            buffer,
        }
    }

    pub fn from_mask(image: GrayImage) -> Self
    {
        let (width, height) = image.dimensions();
        let buffer = image.iter().map(|&v| v == 255).collect();
        BinaryImage::from_bitvec(width, height, buffer)
    }

    #[inline]
    #[must_use]
    pub fn get_pixel(&self, x: u32, y: u32) -> Bit {
        GenericImageView::get_pixel(self, x, y)
    }

    #[inline]
    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }
    #[inline]
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }
}

impl GenericImageView for BinaryImage {
    type Pixel = Bit;
    #[inline]
    fn dimensions(&self) -> (u32, u32) {
        (self.width(), self.height())
    }
    #[inline]
    fn width(&self) -> u32 {
        self.width
    }
    #[inline]
    fn height(&self) -> u32 {
        self.height
    }
    #[inline]
    fn get_pixel(&self, x: u32, y: u32) -> Self::Pixel {
        debug_assert!(self.in_bounds(x, y));
        unsafe { self.unsafe_get_pixel(x, y) }
    }
    #[inline]
    unsafe fn unsafe_get_pixel(&self, x: u32, y: u32) -> Self::Pixel {
        unsafe {
            Bit::from(self.buffer.get_unchecked((y * self.width + x) as usize))
        }
    }
}

impl GenericImage for BinaryImage {
    fn get_pixel_mut(&mut self, _: u32, _: u32) -> &mut Self::Pixel {
        unimplemented!()
    }
    #[inline]
    fn put_pixel(&mut self, x: u32, y: u32, pixel: Self::Pixel) {
        debug_assert!(self.in_bounds(x, y));
        unsafe { self.unsafe_put_pixel(x, y, pixel) }
    }
    #[inline]
    unsafe fn unsafe_put_pixel(&mut self, x: u32, y: u32, pixel: Self::Pixel) {
        self.buffer.set((y * self.width + x) as usize, *pixel);
    }
    fn blend_pixel(&mut self, x: u32, y: u32, other: Self::Pixel) {
        let pixel = self.get_pixel(x, y);
        self.put_pixel(x, y, pixel | other);
    }
}

impl From<image::DynamicImage> for BinaryImage {
    fn from(image: image::DynamicImage) -> Self {
        match image {
            image::DynamicImage::ImageRgb8(image) => Self::from(image),
            image::DynamicImage::ImageLuma8(image) => Self::from(image),
            image::DynamicImage::ImageRgba8(image) => Self::from(image),
            image::DynamicImage::ImageRgb16(image) => Self::from(image),
            image::DynamicImage::ImageLumaA8(image) => Self::from(image),
            image::DynamicImage::ImageLuma16(image) => Self::from(image),
            image::DynamicImage::ImageRgba16(image) => Self::from(image),
            image::DynamicImage::ImageRgb32F(image) => Self::from(image),
            image::DynamicImage::ImageLumaA16(image) => Self::from(image),
            image::DynamicImage::ImageRgba32F(image) => Self::from(image),
            _ => unimplemented!(),
        }
    }
}

impl<Container, P> From<image::ImageBuffer<P, Container>> for BinaryImage
where
    Container: Deref<Target = [P::Subpixel]>,
    P: Pixel,
{
    fn from(image: image::ImageBuffer<P, Container>) -> Self {
        let buffer = image.pixels().map(|pixel| {
            pixel.to_rgba().0[3].to_u8() > Some(77)
        }).collect();
        BinaryImage::from_bitvec(image.width(), image.height(), buffer)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryView<'a, I: GenericImageView> {
    Ref(&'a I),
    Image(I),
}

impl<I, P> GenericImageView for BinaryView<'_, I>
where
    I: GenericImageView<Pixel = P>,
    P: Pixel,
    Bit: From<P>
{
    type Pixel = Bit;
    #[inline]
    fn dimensions(&self) -> (u32, u32) {
        self.deref().dimensions()
    }
    #[inline]
    fn width(&self) -> u32 {
        self.deref().width()
    }
    #[inline]
    fn height(&self) -> u32 {
        self.deref().height()
    }
    #[inline]
    fn get_pixel(&self, x: u32, y: u32) -> Self::Pixel {
        debug_assert!(self.deref().in_bounds(x, y), "Pixel out of bounds");
        unsafe { self.unsafe_get_pixel(x, y) }
    }
    #[inline]
    unsafe fn unsafe_get_pixel(&self, x: u32, y: u32) -> Self::Pixel {
        unsafe {
            Bit::from(self.deref().unsafe_get_pixel(x, y))
        }
    }
}

impl<I, P> Deref for BinaryView<'_, I>
where
    I: GenericImageView<Pixel = P>,
    P: Pixel,
{
    type Target = I;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Ref(image) => image,
            Self::Image(image) => image,
        }
    }
}

impl<'a, I, P> From<BinaryView<'a, I>> for BinaryImage
where
    I: GenericImageView<Pixel = P>,
    P: Pixel,
    Bit: From<P>
{
    fn from(view: BinaryView<'a, I>) -> BinaryImage {
        BinaryImage {
            height: view.height(),
            width: view.width(),
            buffer: view.pixels().map(|(_, _, pixel)| *pixel).collect(),
        }
    }
}