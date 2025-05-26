use mimesis::draw::DrawMesh;
use mimesis::mesh::PolygonMesh;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use image::{Luma, ImageBuffer, GenericImageView, DynamicImage, ImageEncoder, ExtendedColorType, ImageResult};
use geo::{ChaikinSmoothing, Polygon, Simplify};
use mimesis::BinaryImage;
use clap::{Parser, ValueEnum};
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use serde::{Deserialize, Serialize};
use std::io::Write;

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
    #[arg(short, long, default_value = "false")]
    verbose: bool,
}

#[derive(Clone, ValueEnum, Debug, Serialize, Deserialize)]
enum MaskMethod {
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

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    /// Innput setting
    pub input: InputConfig,
    /// Processing parameters
    pub processing: ProcessingConfig,
    /// Batch processing settings
    pub batch: BatchConfig,
    /// Output settings
    pub output: OutputConfig,
}

#[derive(Debug, Serialize, Deserialize)]
struct InputConfig {
    /// Input image file path or directory path
    pub input: PathBuf,
    /// Optional binary mask image path
    pub mask: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProcessingConfig {
    /// Simplification tolerance for Ramer-Douglas-Peucker algorithm
    pub simplify_tolerance: f64,
    /// Number of Chaikin smoothing iterations
    pub smooth_iterations: usize,
    /// Extrusion height for 3D mesh
    pub extrude_height: f64,
    /// Minimum polygon dimension (in pixels)
    pub min_polygon_dimension: usize,
    /// Threshold for binary mask generation (0-255)
    pub threshold: u8,
    /// Method for generating binary mask from texture
    pub mask_method: MaskMethod,
}

#[derive(Debug, Serialize, Deserialize)]
struct BatchConfig {
    /// File patterns to include in batch processing
    pub include_patterns: Vec<String>,
    /// File patterns to exclude from batch processing
    pub exclude_patterns: Vec<String>,
    /// Number of parallel workers for batch processing
    pub workers: usize,
    /// Continue batch processing even if some files fail
    pub continue_on_error: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OutputConfig {
    /// Output folder for processed files
    pub output_folder: PathBuf,
    /// Side texture path for OBJ export
    pub side_texture: Option<PathBuf>,
    /// Back texture path for OBJ export
    pub back_texture: Option<PathBuf>,
    /// Skip saving intermediate polygon images
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
        get_extended_color_type(image),
    )
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

fn process_single_file(
    texture_path: &Path,
    mask_path: Option<&Path>,
    output_dir: &Path,
    config: &Config,
    verbose: bool,
) -> Result<usize, Box<dyn std::error::Error>> {
    let texture_image = image::open(texture_path)
        .map_err(|e| format!("Failed to open texture image: {}", e))?;

    let (width, height) = texture_image.dimensions();
    let asset_name = texture_path.file_stem()
        .ok_or("Invalid texture filename")?
        .to_string_lossy();

    if verbose {
        println!("Processing: {} ({}x{} pixels)", texture_path.display(), width, height);
    }

    // Create or load binary mask
    let binary = if let Some(mask_path) = mask_path {
        if verbose {
            println!("Loading mask from: {}", mask_path.display());
        }
        let mask_image = image::open(mask_path)
            .map_err(|e| format!("Failed to open mask image: {}", e))?;
        BinaryImage::from_mask(mask_image.to_luma8())
    } else {
        if verbose {
            println!("Generating mask using {:?} method", config.processing.mask_method);
        }
        generate_binary_mask(&texture_image, &config.processing.mask_method, config.processing.threshold)
    };

    // Create output directory
    let file_output_dir = output_dir.to_path_buf();
    let textures_output_dir = file_output_dir.join("textures");

    fs::create_dir_all(&textures_output_dir)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    // Save original texture image
    let front_texture_filename = format!("{}.png", asset_name);
    let texture_path = textures_output_dir.join(&front_texture_filename);
    save_uncompressed_png(&texture_path, &texture_image)?;

    let side_texture_filename = if let Some(side_texture_path) = &config.output.side_texture {
        let filename = "side.png".to_string();
        fs::copy(&side_texture_path, textures_output_dir.join(&filename))
            .map_err(|e| format!("Failed to copy side texture: {}", e))?;
        filename
    } else {
        front_texture_filename.clone()
    };

    let back_texture_filename = if let Some(back_texture_path) = &config.output.back_texture {
        let filename = "back.png".to_string();
        fs::copy(&back_texture_path, textures_output_dir.join(&filename))
            .map_err(|e| format!("Failed to copy back texture: {}", e))?;
        filename
    } else {
        front_texture_filename.clone()
    };

    // Save binary mask visualization
    if !config.output.skip_intermediates {
        let visual = ImageBuffer::from_fn(binary.width(), binary.height(), |x, y| {
            let pixel = binary.get_pixel(x, y);
            if *pixel {
                Luma([255u8])
            } else {
                Luma([0u8])
            }
        });

        let mask_path = file_output_dir.join(format!("{}_mask.png", asset_name));
        save_uncompressed_png(&mask_path, &DynamicImage::ImageLuma8(visual))?;
    }

    // Convert binary mask to polygons
    let polygons: Vec<Polygon> = binary.trace_polygons(config.processing.min_polygon_dimension);

    if verbose {
        println!("Found {} polygons for {}", polygons.len(), asset_name);
    }

    // Process polygons
    for (i, polygon) in polygons.iter().enumerate() {
        if !config.output.skip_intermediates {
            let result_img = polygon.draw(width, height);
            let polygon_path = file_output_dir.join(format!("{}_polygon_{}.png", asset_name, i));
            result_img.save(&polygon_path)
                .map_err(|e| format!("Failed to save polygon image: {}", e))?;
        }
    }

    // Simplify polygons
    let simplified_polygons: Vec<Polygon> = if config.processing.simplify_tolerance <= 0f64 {
        polygons
    }
    else {
        let mut simplified_polygons: Vec<Polygon> = Vec::new();
        for polygon in polygons.iter() {
            let simplified_polygon = polygon.simplify(&config.processing.simplify_tolerance);
            simplified_polygons.push(simplified_polygon);
        }
        simplified_polygons
    };

    // Smooth polygons
    let smooth_polygons: Vec<Polygon> = if config.processing.smooth_iterations <= 0 {
        simplified_polygons
    }
    else {
        let mut smooth_polygons: Vec<Polygon> = Vec::new();
        for polygon in simplified_polygons.iter() {
            let smooth_polygon = polygon.chaikin_smoothing(config.processing.smooth_iterations);
            smooth_polygons.push(smooth_polygon);
        }
        smooth_polygons
    };
    
    // Create meshes
    for (i, polygon) in smooth_polygons.iter().enumerate() {
        // Create 2D mesh
        let mesh2d = polygon.mesh2d()
            .map_err(|e| format!("Failed to create 2D mesh for polygon {}: {}", i, e))?;

        if !config.output.skip_intermediates {
            let mesh2d_path = file_output_dir.join(format!("{}_{}.2d.obj", asset_name, i));
            mesh2d.export_obj(mesh2d_path.as_path())
                .map_err(|e| format!("Failed to export 2D mesh: {}", e))?;
        }

        // Create 3D mesh
        let mesh3d = mesh2d.extrude(config.processing.extrude_height, width as f64, height as f64);
        let mesh_path = file_output_dir.join(format!("{}_{}.obj", asset_name, i));
        let material_path = file_output_dir.join(format!("{}_{}.mtl", asset_name, i));

        mesh3d.export_obj(
            mesh_path.as_path(),
            material_path.as_path(),
            &front_texture_filename,
            &back_texture_filename,
            &side_texture_filename
        ).map_err(|e| format!("Failed to export 3D mesh: {}", e))?;
    }

    Ok(smooth_polygons.len())
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

    if args.verbose {
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

        match process_single_file(
            &input_file,
            mask.as_deref(),
            &config.output.output_folder,
            &config,
            args.verbose,
        ) {
            Ok(polygon_count) => {
                stats.processed += 1;
                stats.total_polygons += polygon_count;
                if args.verbose {
                    println!("Successfully processed: {} ({} polygons)",
                             input_file.display(), polygon_count);
                }
            }
            Err(e) => {
                stats.failed += 1;
                eprintln!("Failed to process {}: {}", input_file.display(), e);
                if !config.batch.continue_on_error {
                    return Err(e);
                }
            }
        }

        if args.verbose && input_files.len() > 1 {
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