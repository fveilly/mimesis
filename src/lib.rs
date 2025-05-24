mod binary_image;
mod contour;
mod pixel;
pub mod mesh;
pub mod draw;
mod error;

pub use crate::binary_image::BinaryImage;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
    }
}
