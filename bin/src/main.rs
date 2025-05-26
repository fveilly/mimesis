mod config;
mod processing;

use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use image::DynamicImage;
use mimesis::BinaryImage;
use clap::Parser;
use std::io::Write;
use crate::config::{Config, MaskMethod};
use crate::processing::Processor;

#[derive(Parser)]
#[command(name = "mesh-generator")]
#[command(about = "Generate 3D meshes from images using contour tracing")]
#[command(version = "1.0")]
struct Args {
    /// Input texture image path or directory for batch processing
    #[arg(short, long)]
    input: Option<PathBuf>,

    /// Optional binary mask image path (if not provided, mask will be generated from texture)
    #[arg(short, long)]
    mask: Option<PathBuf>,

    /// Output directory
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Configuration file path
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Generate default configuration file and exit
    #[arg(long)]
    generate_config: bool,

    /// File patterns to include in batch processing (e.g., "*.png,*.jpg")
    #[arg(long, default_value = "*.png,*.jpg,*.jpeg,*.bmp,*.tiff,*.tga")]
    include_patterns: Option<String>,

    /// File patterns to exclude from batch processing
    #[arg(long)]
    exclude_patterns: Option<String>,

    /// Number of parallel workers for batch processing
    #[arg(long)]
    workers: Option<usize>,

    /// Continue batch processing even if some files fail
    #[arg(long)]
    continue_on_error: Option<bool>,

    /// Simplification tolerance for Ramer-Douglas-Peucker algorithm
    #[arg(long)]
    simplify_tolerance: Option<f64>,

    /// Number of Chaikin smoothing iterations
    #[arg(long)]
    smooth_iterations: Option<usize>,

    /// Extrusion height for 3D mesh
    #[arg(long)]
    extrude_height: Option<f64>,

    /// Minimum polygon dimension (in pixels)
    #[arg(long)]
    min_polygon_dimension: Option<usize>,

    /// Threshold for binary mask generation (0-255)
    #[arg(long)]
    threshold: Option<u8>,

    /// Method for generating binary mask from texture
    #[arg(long)]
    mask_method: Option<MaskMethod>,

    /// Side texture file name for OBJ export
    #[arg(long)]
    side_texture: Option<PathBuf>,

    /// Back texture file name for OBJ export
    #[arg(long)]
    back_texture: Option<PathBuf>,

    /// Skip saving intermediate polygon images
    #[arg(long)]
    skip_intermediates: Option<bool>,

    /// Verbose output
    #[arg(long)]
    verbose: bool,
}

fn load_config(config_path: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    let config_str = fs::read_to_string(config_path)?;
    let config: Config = match config_path.extension().and_then(|s| s.to_str()) {
        Some("json") => serde_json::from_str(&config_str)?,
        Some("toml") => toml::from_str(&config_str)?,
        _ => return Err("Unsupported config file format. Use .json, .toml, or .yaml".into()),
    };
    Ok(config)
}

fn save_default_config(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::default();
    let config_str = match path.extension().and_then(|s| s.to_str()) {
        Some("json") => serde_json::to_string_pretty(&config)?,
        Some("toml") => toml::to_string_pretty(&config)?,
        _ => serde_json::to_string_pretty(&config)?, // Default to JSON
    };

    let mut file = File::create(path)?;
    file.write_all(config_str.as_bytes())?;
    println!("Generated default configuration file: {}", path.display());
    Ok(())
}

fn matches_patterns(filename: &str, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }

    patterns.iter().any(|pattern| {
        if pattern.contains('*') {
            // Simple glob matching
            let pattern = pattern.replace('*', "");
            if pattern.starts_with('.') {
                filename.ends_with(&pattern)
            } else {
                filename.contains(&pattern)
            }
        } else {
            filename == pattern
        }
    })
}

fn find_input_files(
    input_path: &Path,
    include_patterns: &[String],
    exclude_patterns: &[String],
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();

    if input_path.is_file() {
        files.push(input_path.to_path_buf());
    } else if input_path.is_dir() {
        for entry in fs::read_dir(input_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    let matches_include = matches_patterns(filename, include_patterns);
                    let matches_exclude = matches_patterns(filename, exclude_patterns);

                    if matches_include && !matches_exclude {
                        files.push(path);
                    }
                }
            }
        }
    }

    files.sort();
    Ok(files)
}

#[derive(Debug)]
struct ProcessingStats {
    total_files: usize,
    processed: usize,
    failed: usize,
    total_polygons: usize,
}

impl ProcessingStats {
    fn new(total_files: usize) -> Self {
        Self {
            total_files,
            processed: 0,
            failed: 0,
            total_polygons: 0,
        }
    }

    fn print_progress(&self) {
        println!(
            "Progress: {}/{} files processed, {} failed, {} polygons generated",
            self.processed + self.failed,
            self.total_files,
            self.failed,
            self.total_polygons
        );
    }

    fn print_summary(&self) {
        println!("\n=== Processing Summary ===");
        println!("Total files: {}", self.total_files);
        println!("Successfully processed: {}", self.processed);
        println!("Failed: {}", self.failed);
        println!("Total polygons generated: {}", self.total_polygons);
        println!("Success rate: {:.1}%",
                 (self.processed as f64 / self.total_files as f64) * 100.0);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Handle config generation
    if args.generate_config {
        let config_path = args.config.unwrap_or_else(|| PathBuf::from("mesh_config.json"));
        save_default_config(&config_path)?;
        return Ok(());
    }

    // Load configuration
    let mut config = if let Some(config_path) = &args.config {
        load_config(config_path)?
    } else {
        Config::default()
    };

    // Override config with command line arguments
    if let Some(input) = args.input {
        config.input.input = input;
    }
    if args.mask.is_some() {
        config.input.mask = args.mask;
    }
    if let Some(simplify_tolerance) = args.simplify_tolerance {
        config.processing.simplify_tolerance = simplify_tolerance;
    }
    if let Some(smooth_iterations) = args.smooth_iterations {
        config.processing.smooth_iterations = smooth_iterations;
    }
    if let Some(extrude_height) = args.extrude_height {
        config.processing.extrude_height = extrude_height;
    }
    if let Some(threshold) = args.threshold {
        config.processing.threshold = threshold;
    }
    if args.verbose {
        config.processing.verbose = true;
    }
    if let Some(workers) = args.workers {
        config.batch.workers = workers;
    }
    if let Some(output) = args.output {
        config.output.output_folder = output;
    }
    if let Some(continue_on_error) = args.continue_on_error {
        config.batch.continue_on_error = continue_on_error;
    }
    if let Some(skip_intermediates) = args.skip_intermediates {
        config.output.skip_intermediates = skip_intermediates;
    }

    // Parse include patterns from command line
    if let Some(include_patterns) = args.include_patterns {
        let mut include_patterns: Vec<String> = include_patterns
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        config.batch.include_patterns.append(&mut include_patterns);
    }
    if let Some(exclude_patterns) = args.exclude_patterns {
        let mut exclude_patterns: Vec<String> = exclude_patterns
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        config.batch.exclude_patterns.append(&mut exclude_patterns);
    }

    config.batch.exclude_patterns.push("*_mask*".to_string());

    // Find input files
    let input_files = find_input_files(&config.input.input, &config.batch.include_patterns, &config.batch.exclude_patterns)?;

    if input_files.is_empty() {
        eprintln!("No input files found matching the criteria");
        std::process::exit(1);
    }

    let batch = config.input.input.is_dir();

    if config.processing.verbose {
        println!("Found {} input files", input_files.len());
        for file in &input_files {
            println!("  - {}", file.display());
        }
    }

    // Create output directory
    fs::create_dir_all(&config.output.output_folder)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    let mut stats = ProcessingStats::new(input_files.len());

    // Process files
    if config.batch.workers > 1 {
        // TODO: Implement parallel processing using rayon or similar
        println!("Parallel processing not yet implemented, processing sequentially...");
    }

    let processor = Processor::new(config.clone());

    // Sequential processing
    for input_file in &input_files {
        let mask = if batch {
            let stem = input_file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let mask_filename = format!("{}_mask.png", stem);
            let mask_path = input_file.with_file_name(mask_filename);
            if !mask_path.exists() { None } else { Some(mask_path) }
        }
        else {
            config.input.mask.clone()
        };

        match processor.process(&input_file, mask.as_deref()) {
            Ok(polygon_count) => {
                stats.processed += 1;
                stats.total_polygons += polygon_count;
                if config.processing.verbose {
                    println!("Successfully processed: {} ({} polygons)",
                             input_file.display(), polygon_count);
                }
            }
            Err(e) => {
                stats.failed += 1;
                eprintln!("Failed to process {}: {}", input_file.display(), e);
                if !config.batch.continue_on_error {
                    return Err(e.into());
                }
            }
        }

        if config.processing.verbose && input_files.len() > 1 {
            stats.print_progress();
        }
    }

    // Print final summary
    if input_files.len() > 1 {
        stats.print_summary();
    } else {
        println!("Successfully generated {} meshes in: {}",
                 stats.total_polygons, config.output.output_folder.display());
    }

    Ok(())
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