use std::path::Path;
use anyhow::anyhow;
use fast_image_resize::images::Image;
use fast_image_resize::{FilterType, MulDiv, PixelType, ResizeAlg, ResizeOptions, Resizer};
use image::{DynamicImage, ImageBuffer, Rgb};
use ort::session::{Session};
use ndarray::{s, Array3, ArrayView, Axis, Dim};
use ort::inputs;
use crate::BinaryImage;

const ML_MODEL_IMAGE_WIDTH: u32 = 1024;
const ML_MODEL_IMAGE_HEIGHT: u32 = 1024;
const ML_MODEL_INPUT_NAME: &str = "input";
const ML_MODEL_OUTPUT_NAME: &str = "output";

pub struct BackgroundRemover {
    model: Session,
}

impl BackgroundRemover {

    pub fn new(model_path: impl AsRef<Path>) -> Result<Self, ort::Error> {
        let model = Session::builder()?.commit_from_file(model_path)?;
        Ok(BackgroundRemover { model })
    }

    pub fn remove_background(&self, original_img: &DynamicImage) -> anyhow::Result<BinaryImage> {
        let img = Self::preprocess_image(original_img)?;

        let input = img.insert_axis(Axis(0));
        let inputs = inputs![ML_MODEL_INPUT_NAME => input.view()]?;

        let outputs = self.model.run(inputs)?;

        let output = outputs[ML_MODEL_OUTPUT_NAME].try_extract_tensor()?;
        let view = output.view();
        let output: ArrayView<f32, Dim<[usize; 2]>> = view.slice(s![0, 0, .., ..]);

        let image = Self::postprocess_image(&output)?;

        let (original_width, original_height) = (original_img.width(), original_img.height());
        let resized = Self::resize_rgba(&image, original_width, original_height)?;
        let mask = BinaryImage::from_raw(original_width, original_height, &resized);
        Ok(mask)
    }

    fn preprocess_image(image: &DynamicImage) -> anyhow::Result<Array3<f32>> {
        let img_vec = Self::resize_rgba(image, ML_MODEL_IMAGE_WIDTH, ML_MODEL_IMAGE_HEIGHT)?;

        // Separate R, G, and B components
        let mut r_vec = Vec::with_capacity((ML_MODEL_IMAGE_WIDTH * ML_MODEL_IMAGE_HEIGHT) as usize);
        let mut g_vec = Vec::with_capacity((ML_MODEL_IMAGE_WIDTH * ML_MODEL_IMAGE_HEIGHT) as usize);
        let mut b_vec = Vec::with_capacity((ML_MODEL_IMAGE_WIDTH * ML_MODEL_IMAGE_HEIGHT) as usize);

        for chunk in img_vec.chunks(4) {
            r_vec.push(chunk[0]);
            g_vec.push(chunk[1]);
            b_vec.push(chunk[2]);
            // SKIP Alpha channel
        }

        // Concatenate R, G, and B vectors to form the correctly ordered vector
        let reordered_vec = [r_vec, g_vec, b_vec].concat();

        // Convert the resized image to a ndarray.
        let img_ndarray = Array3::from_shape_vec(
            (
                3,
                ML_MODEL_IMAGE_WIDTH as usize,
                ML_MODEL_IMAGE_HEIGHT as usize,
            ),
            reordered_vec,
        )?;

        // Convert to floating point and scale pixel values to [0, 1].
        let img_float: Array3<f32> = img_ndarray.mapv(|x| x as f32 / 255.0);

        // Normalize the image.
        Ok(Self::normalize_image(&img_float))
    }

    fn normalize_image(img: &Array3<f32>) -> Array3<f32> {
        // The mean and std are applied across the channel dimension.
        let mean = Array3::from_elem((1, img.shape()[1], img.shape()[2]), 0.5);
        let std = Array3::from_elem((1, img.shape()[1], img.shape()[2]), 1.0);

        // Broadcasting the mean and std to match img dimensions and applying normalization.
        (img - &mean) / &std
    }

    fn postprocess_image(
        model_result: &ArrayView<f32, Dim<[usize; 2]>>,
    ) -> anyhow::Result<DynamicImage> {
        let ma = model_result
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .ok_or(anyhow!("Should be OK"))?;
        let mi = model_result
            .iter()
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .ok_or(anyhow!("Should be OK"))?;
        let result = (model_result.mapv(|x| x - mi) / (ma - mi)) * 255.0;

        let result_u8 = result.mapv(|x| x as u8).into_raw_vec_and_offset();

        let mut imgbuf: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::new(ML_MODEL_IMAGE_WIDTH, ML_MODEL_IMAGE_HEIGHT);

        for (x, y, pixel) in imgbuf.enumerate_pixels_mut() {
            let index = (y * ML_MODEL_IMAGE_WIDTH + x) as usize;
            let value = result_u8.0[index];
            *pixel = Rgb([value, value, value]);
        }

        Ok(DynamicImage::ImageRgb8(imgbuf))
    }

    pub fn resize_rgba(
        img: &DynamicImage,
        target_width: u32,
        target_height: u32,
    ) -> anyhow::Result<Vec<u8>> {
        let rgba_data = img.to_rgba8().into_raw();
        let mut src_image = Image::from_vec_u8(
            img.width(),
            img.height(),
            rgba_data,
            PixelType::U8x4,
        )?;

        // Pre-multiply alpha
        let alpha_mul_div = MulDiv::default();
        alpha_mul_div.multiply_alpha_inplace(&mut src_image)?;

        // Destination image
        let mut dst_image = Image::new(target_width, target_height, PixelType::U8x4);

        // Create resizer and set algorithm
        let mut resizer = Resizer::new();
        let mut resize_option = ResizeOptions::new();
        resize_option.algorithm = ResizeAlg::Convolution(FilterType::Bilinear);

        // Resize operation
        resizer.resize(
            &src_image,
            &mut dst_image,
            Some(&resize_option),
        )?;

        // Un-premultiply alpha
        alpha_mul_div.divide_alpha_inplace(&mut dst_image)?;

        Ok(dst_image.into_vec())
    }
    
}