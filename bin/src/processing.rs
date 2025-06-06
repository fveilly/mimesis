use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use anyhow::anyhow;
use geo::{ChaikinSmoothing, Polygon, Simplify};
use image::{DynamicImage, ExtendedColorType, GenericImageView, ImageBuffer, ImageEncoder, ImageResult, Luma};
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use mimesis::{BinaryImage};
use mimesis::draw::DrawMesh;
use mimesis::mesh::PolygonMesh;
use crate::config::{Config, MaskMethod};
use crate::stats::{Benchmark, MeshStats, ProcessingResult};

#[cfg(feature = "background-remover")]
use mimesis::BackgroundRemover;

pub(crate) struct Processor {
    config: Config
}

impl Processor {

    pub(crate) fn new(config: Config) -> Self {
        Processor {
            config
        }
    }

    #[cfg(feature = "background-remover")]
    #[inline]
    fn background_removal_feature_supported(&self) -> bool {
        return self.config.processing.use_onnx_background_removal
    }
    #[cfg(not(feature = "background-remover"))]
    #[inline]
    fn background_removal_feature_supported(&self) -> bool {
        false
    }

    pub(crate) fn process(&self, input: &PathBuf, mask: Option<&Path>) -> anyhow::Result<ProcessingResult> {
        let mut benchmarks = Benchmark::now();

        // Step 1: Load texture image
        let texture_image = image::open(input)
            .map_err(|e| anyhow!(format!("Failed to open texture image: {}", e)))?;
        benchmarks.step( "Load texture image");

        let (width, height) = texture_image.dimensions();
        let asset_name = input.file_stem()
            .ok_or(anyhow!("Invalid texture filename"))?
            .to_string_lossy();

        // Step 2: Create or load binary mask
        let binary = if let Some(mask_path) = mask {
            let mask_image = image::open(mask_path)
                .map_err(|e| anyhow!(format!("Failed to open mask image: {}", e)))?;
            BinaryImage::from_mask(mask_image.to_luma8())
        } else if self.background_removal_feature_supported() {
            #[cfg(feature = "background-remover")]
            if let Some(onnx_model_path) = &self.config.processing.onnx_model_path {
                let background_remover = BackgroundRemover::new(onnx_model_path)?;
                background_remover.remove_background(&texture_image)
                    .map_err(|e| anyhow!(format!("Failed to remove background with ONNX: {}", e)))?
            } else {
                return Err(anyhow!("ONNX background removal enabled but no model path provided"));
            }
            #[cfg(not(feature = "background-remover"))]
            unreachable!()
        } else {
            Self::generate_binary_mask(&texture_image, &self.config.processing.mask_method, self.config.processing.threshold)
        };
        benchmarks.step( "Generate/load mask");

        // Step 3: Setup output directories and save textures
        let file_output_dir = self.config.output.output_folder.to_path_buf();
        let textures_output_dir = file_output_dir.join("textures");

        fs::create_dir_all(&textures_output_dir)
            .map_err(|e| anyhow!(format!("Failed to create output directory: {}", e)))?;

        // Save original texture image
        let front_texture_filename = format!("{}.png", asset_name);
        let texture_path = textures_output_dir.join(&front_texture_filename);
        Self::save_uncompressed_png(&texture_path, &texture_image)
            .map_err(|e| anyhow!(format!("Failed to save texture: {}", e)))?;

        let side_texture_filename = if let Some(side_texture_path) = &self.config.output.side_texture {
            let filename = "side.png".to_string();
            fs::copy(&side_texture_path, textures_output_dir.join(&filename))
                .map_err(|e| anyhow!(format!("Failed to copy side texture: {}", e)))?;
            filename
        } else {
            front_texture_filename.clone()
        };

        let back_texture_filename = if let Some(back_texture_path) = &self.config.output.back_texture {
            let filename = "back.png".to_string();
            fs::copy(&back_texture_path, textures_output_dir.join(&filename))
                .map_err(|e| anyhow!(format!("Failed to copy back texture: {}", e)))?;
            filename
        } else {
            front_texture_filename.clone()
        };
        benchmarks.step( "Save front and back textures");

        // Step 4: Save binary mask visualization
        if !self.config.output.skip_intermediates {
            let visual = ImageBuffer::from_fn(binary.width(), binary.height(), |x, y| {
                let pixel = binary.get_pixel(x, y);
                if *pixel {
                    Luma([255u8])
                } else {
                    Luma([0u8])
                }
            });

            let mask_path = file_output_dir.join(format!("{}_mask.png", asset_name));
            Self::save_uncompressed_png(&mask_path, &DynamicImage::ImageLuma8(visual))
                .map_err(|e| anyhow!(format!("Failed to save mask: {}", e)))?;
            benchmarks.step( "Save mask visualization");
        }

        // Step 5: Convert binary mask to polygons
        let polygons: Vec<Polygon> = binary.trace_polygons(self.config.processing.min_polygon_dimension);
        benchmarks.step( "Trace polygons");

        // Step 6: Process polygon visualization
        if !self.config.output.skip_intermediates {
            for (i, polygon) in polygons.iter().enumerate() {
                let result_img = polygon.draw(width, height);
                let polygon_path = file_output_dir.join(format!("{}_polygon_{}.png", asset_name, i));
                result_img.save(&polygon_path)
                    .map_err(|e| anyhow!(format!("Failed to save polygon image: {}", e)))?;
            }
            benchmarks.step( "Save polygon visualizations");
        }

        // Step 7: Simplify polygons
        let simplified_polygons: Vec<Polygon> = if self.config.processing.simplify_tolerance <= 0f64 {
            polygons
        }
        else {
            let mut simplified_polygons: Vec<Polygon> = Vec::new();
            for polygon in polygons.iter() {
                let simplified_polygon = polygon.simplify(&self.config.processing.simplify_tolerance);
                simplified_polygons.push(simplified_polygon);
            }
            simplified_polygons
        };
        benchmarks.step( "Simplify polygons");

        // Step 8: Smooth polygons
        let smooth_polygons: Vec<Polygon> = if self.config.processing.smooth_iterations <= 0 {
            simplified_polygons
        }
        else {
            let mut smooth_polygons: Vec<Polygon> = Vec::new();
            for polygon in simplified_polygons.iter() {
                let smooth_polygon = polygon.chaikin_smoothing(self.config.processing.smooth_iterations);
                smooth_polygons.push(smooth_polygon);
            }
            smooth_polygons
        };
        benchmarks.step( "Smooth polygons");

        // Step 9: Create meshes
        let mut mesh_stats = Vec::new();

        for (i, polygon) in smooth_polygons.iter().enumerate() {
            // Create 2D mesh
            let mesh2d = polygon.mesh2d()
                .map_err(|e| anyhow!(format!("Failed to create 2D mesh for polygon {}: {}", i, e)))?;

            let vertex_count_2d = mesh2d.get_vertices().len();
            let triangle_count_2d = mesh2d.get_indices().len() / 3;

            if !self.config.output.skip_intermediates {
                let mesh2d_path = file_output_dir.join(format!("{}_{}.2d.obj", asset_name, i));
                mesh2d.export_obj(mesh2d_path.as_path())
                    .map_err(|e| anyhow!(format!("Failed to export 2D mesh: {}", e)))?;
            }

            // Create 3D mesh
            let mesh3d = mesh2d.extrude(self.config.processing.extrude_height, width as f64, height as f64);
            let vertex_count_3d = mesh3d.get_vertices().len();
            let triangle_count_3d = mesh3d.get_faces().iter().map(|group| group.indices.len()).sum();

            let mesh_path = file_output_dir.join(format!("{}_{}.obj", asset_name, i));
            let material_path = file_output_dir.join(format!("{}_{}.mtl", asset_name, i));

            mesh3d.export_obj(
                mesh_path.as_path(),
                material_path.as_path(),
                &front_texture_filename,
                &back_texture_filename,
                &side_texture_filename
            ).map_err(|e| anyhow!(format!("Failed to export 3D mesh: {}", e)))?;

            mesh_stats.push(MeshStats {
                vertex_count_2d,
                triangle_count_2d,
                vertex_count_3d,
                triangle_count_3d,
            });
        }
        benchmarks.step( "Generate meshes");

        let total_duration = benchmarks.get_total_duration();

        Ok(ProcessingResult {
            input: input.to_path_buf(), 
            width, height,
            polygon_count: smooth_polygons.len(),
            mesh_stats,
            benchmarks,
            total_duration,
        })
    }

    fn generate_binary_mask(image: &DynamicImage, method: &MaskMethod, threshold: u8) -> BinaryImage {
        match method {
            MaskMethod::Luminance => {
                let gray = image.to_luma8();
                let binary_data: Vec<u8> = gray.pixels()
                    .map(|pixel| if pixel.0[0] > threshold { 255 } else { 0 })
                    .collect();
                BinaryImage::from_raw(gray.width(), gray.height(), &binary_data)
            },
            MaskMethod::Alpha => {
                let rgba = image.to_rgba8();
                let binary_data: Vec<u8> = rgba.pixels()
                    .map(|pixel| if pixel.0[3] > threshold { 255 } else { 0 })
                    .collect();
                BinaryImage::from_raw(rgba.width(), rgba.height(), &binary_data)
            },
            MaskMethod::Red => {
                let rgb = image.to_rgb8();
                let binary_data: Vec<u8> = rgb.pixels()
                    .map(|pixel| if pixel.0[0] > threshold { 255 } else { 0 })
                    .collect();
                BinaryImage::from_raw(rgb.width(), rgb.height(), &binary_data)
            },
            MaskMethod::Green => {
                let rgb = image.to_rgb8();
                let binary_data: Vec<u8> = rgb.pixels()
                    .map(|pixel| if pixel.0[1] > threshold { 255 } else { 0 })
                    .collect();
                BinaryImage::from_raw(rgb.width(), rgb.height(), &binary_data)
            },
            MaskMethod::Blue => {
                let rgb = image.to_rgb8();
                let binary_data: Vec<u8> = rgb.pixels()
                    .map(|pixel| if pixel.0[2] > threshold { 255 } else { 0 })
                    .collect();
                BinaryImage::from_raw(rgb.width(), rgb.height(), &binary_data)
            },
        }
    }

    fn get_extended_color_type(image: &DynamicImage) -> ExtendedColorType {
        match image {
            DynamicImage::ImageLuma8(_) => ExtendedColorType::L8,
            DynamicImage::ImageLumaA8(_) => ExtendedColorType::La8,
            DynamicImage::ImageRgb8(_) => ExtendedColorType::Rgb8,
            DynamicImage::ImageRgba8(_) => ExtendedColorType::Rgba8,
            DynamicImage::ImageLuma16(_) => ExtendedColorType::L16,
            DynamicImage::ImageLumaA16(_) => ExtendedColorType::La16,
            DynamicImage::ImageRgb16(_) => ExtendedColorType::Rgb16,
            DynamicImage::ImageRgba16(_) => ExtendedColorType::Rgba16,
            _ => panic!("Unsupported DynamicImage format"),
        }
    }

    fn save_uncompressed_png<P: AsRef<Path>>(
        path: P,
        image: &DynamicImage,
    ) -> ImageResult<()> {
        let file = File::create(path)?;
        let encoder = PngEncoder::new_with_quality(
            file,
            CompressionType::Best,
            FilterType::NoFilter,
        );
        encoder.write_image(
            image.as_bytes(),
            image.width(),
            image.height(),
            Self::get_extended_color_type(image),
        )
    }
}