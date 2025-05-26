use std::path::PathBuf;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, ValueEnum, Debug, Serialize, Deserialize)]
pub(crate) enum MaskMethod {
    /// Use luminance/brightness to generate mask
    Luminance,
    /// Use alpha channel to generate mask
    Alpha,
    /// Use red channel to generate mask
    Red,
    /// Use green channel to generate mask
    Green,
    /// Use blue channel to generate mask
    Blue,
}

impl Default for MaskMethod {
    fn default() -> Self {
        MaskMethod::Alpha
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Config {
    /// Input setting
    pub input: InputConfig,
    /// Processing parameters
    pub processing: ProcessingConfig,
    /// Batch processing settings
    pub batch: BatchConfig,
    /// Output settings
    pub output: OutputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct InputConfig {
    /// Input image file path or directory path
    pub input: PathBuf,
    /// Optional binary mask image path
    pub mask: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ProcessingConfig {
    /// Simplification tolerance for Ramer-Douglas-Peucker algorithm
    #[serde(default)]
    pub simplify_tolerance: f64,
    /// Number of Chaikin smoothing iterations
    #[serde(default)]
    pub smooth_iterations: usize,
    /// Extrusion height for 3D mesh
    #[serde(default)]
    pub extrude_height: f64,
    /// Minimum polygon dimension (in pixels)
    #[serde(default)]
    pub min_polygon_dimension: usize,
    /// Threshold for binary mask generation (0-255)
    #[serde(default)]
    pub threshold: u8,
    /// Method for generating binary mask from texture
    #[serde(default)]
    pub mask_method: MaskMethod,
    /// Enable verbose output
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct BatchConfig {
    /// File patterns to include in batch processing
    #[serde(default)]
    pub include_patterns: Vec<String>,
    /// File patterns to exclude from batch processing
    #[serde(default)]
    pub exclude_patterns: Vec<String>,
    /// Number of parallel workers for batch processing
    #[serde(default)]
    pub workers: usize,
    /// Continue batch processing even if some files fail
    #[serde(default)]
    pub continue_on_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OutputConfig {
    /// Output folder for processed files
    #[serde(default)]
    pub output_folder: PathBuf,
    /// Side texture path for OBJ export
    #[serde(default)]
    pub side_texture: Option<PathBuf>,
    /// Back texture path for OBJ export
    #[serde(default)]
    pub back_texture: Option<PathBuf>,
    /// Skip saving intermediate polygon images
    #[serde(default)]
    pub skip_intermediates: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            input: InputConfig {
                input: PathBuf::from("texture.png"),
                mask: None,
            },
            processing: ProcessingConfig {
                simplify_tolerance: 10.0,
                smooth_iterations: 1,
                extrude_height: 20.0,
                min_polygon_dimension: 0,
                threshold: 128,
                mask_method: MaskMethod::Alpha,
                verbose: false
            },
            batch: BatchConfig {
                include_patterns: vec![
                    "*.png".to_string(),
                    "*.jpg".to_string(),
                    "*.jpeg".to_string(),
                    "*.bmp".to_string(),
                    "*.tiff".to_string(),
                    "*.tga".to_string(),
                ],
                exclude_patterns: vec![],
                workers: 1,
                continue_on_error: false,
            },
            output: OutputConfig {
                output_folder: PathBuf::from("output"),
                side_texture: None,
                back_texture: None,
                skip_intermediates: false,
            },
        }
    }
}