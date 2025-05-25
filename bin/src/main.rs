use mimesis::draw::DrawMesh;
use mimesis::mesh::PolygonMesh;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use image::{Luma, ImageBuffer, GenericImageView, DynamicImage, ImageFormat, ImageEncoder, ExtendedColorType, ImageResult};
use geo::{ChaikinSmoothing, Polygon, Simplify};
use mimesis::BinaryImage;
use clap::{Parser, ValueEnum};
use clap::error::ErrorKind::Format;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};

#[derive(Parser)]
#[command(name = "mesh-generator")]
#[command(about = "Generate 3D meshes from images using contour tracing")]
#[command(version = "1.0")]
struct Args {
    /// Input texture image path
    #[arg(short, long)]
    texture: PathBuf,

    /// Optional binary mask image path (if not provided, mask will be generated from texture)
    #[arg(short, long)]
    mask: Option<PathBuf>,

    /// Output directory
    #[arg(short, long, default_value = "out")]
    output: PathBuf,

    /// Simplification tolerance for Ramer-Douglas-Peucker algorithm
    #[arg(long, default_value = "10.0")]
    simplify_tolerance: f64,

    /// Number of Chaikin smoothing iterations
    #[arg(long, default_value = "1")]
    smooth_iterations: usize,

    /// Extrusion height for 3D mesh
    #[arg(long, default_value = "20.0")]
    extrude_height: f64,

    /// Minimum polygon dimension (in pixels)
    #[arg(long, default_value = "0")]
    min_polygon_dimension: usize,

    /// Threshold for binary mask generation (0-255)
    #[arg(long, default_value = "128")]
    threshold: u8,

    /// Method for generating binary mask from texture
    #[arg(long, default_value = "alpha")]
    mask_method: MaskMethod,

    /// Side texture file name for OBJ export
    #[arg(long, default_value = "side.jpg")]
    side_texture: String,

    /// Back texture file name for OBJ export
    #[arg(long, default_value = "back.jpg")]
    back_texture: String,

    /// Skip saving intermediate polygon images
    #[arg(long)]
    skip_intermediates: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Clone, ValueEnum, Debug)]
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Validate input files
    if !args.texture.exists() {
        eprintln!("Error: Texture file '{}' does not exist", args.texture.display());
        std::process::exit(1);
    }

    if let Some(ref mask_path) = args.mask {
        if !mask_path.exists() {
            eprintln!("Error: Mask file '{}' does not exist", mask_path.display());
            std::process::exit(1);
        }
    }

    // Load texture image
    let texture_image = image::open(&args.texture)
        .map_err(|e| format!("Failed to open texture image: {}", e))?;

    let (width, height) = texture_image.dimensions();
    let asset_name = args.texture.file_stem()
        .ok_or("Invalid texture filename")?
        .to_string_lossy();

    if args.verbose {
        println!("Loaded texture: {}x{} pixels", width, height);
    }

    // Create or load binary mask
    let binary = if let Some(mask_path) = &args.mask {
        if args.verbose {
            println!("Loading mask from: {}", mask_path.display());
        }
        let mask_image = image::open(mask_path)
            .map_err(|e| format!("Failed to open mask image: {}", e))?;
        BinaryImage::from_mask(mask_image.to_luma8())
    } else {
        if args.verbose {
            println!("Generating mask from texture using {:?} method", args.mask_method);
        }
        generate_binary_mask(&texture_image, &args.mask_method, args.threshold)
    };

    // Create output directory
    fs::create_dir_all(&args.output)
        .map_err(|e| format!("Failed to create output directory: {}", e))?;

    // Save original texture image
    let texture_filename = format!("{}_texture.png", asset_name);
    let texture_path = args.output.join(&texture_filename);
    save_uncompressed_png(&texture_path, &texture_image)?;

    // Save binary mask visualization
    if !args.skip_intermediates {
        let visual = ImageBuffer::from_fn(binary.width(), binary.height(), |x, y| {
            let pixel = binary.get_pixel(x, y);
            if *pixel {
                Luma([255u8])
            } else {
                Luma([0u8])
            }
        });

        let mask_path = args.output.join(format!("{}_mask.png", asset_name));
        save_uncompressed_png(&mask_path,  &DynamicImage::ImageLuma8(visual))?;

        if args.verbose {
            println!("Saved binary mask to: {}", mask_path.display());
        }
    }

    // Convert binary mask to polygons using Theo Pavlidis' contour tracing algorithm
    let polygons: Vec<Polygon> = binary.trace_polygons(10);

    if args.verbose {
        println!("Found {} polygons", polygons.len());
    }

    for (i, polygon) in polygons.iter().enumerate() {
        if args.verbose {
            println!("Polygon {}: exterior_points={} interior_rings={}",
                     i,
                     polygon.exterior().points().count(),
                     polygon.interiors().len()
            );
            for (j, interior) in polygon.interiors().iter().enumerate() {
                println!("  interior_{}: points={}", j, interior.points().count());
            }
        }

        if !args.skip_intermediates {
            let result_img = polygon.draw(width, height);
            let polygon_path = args.output.join(format!("{}_polygon_{}.png", asset_name, i));
            result_img.save(&polygon_path)
                .map_err(|e| format!("Failed to save polygon image: {}", e))?;
        }
    }

    // Simplify the polygons using Ramer–Douglas–Peucker algorithm
    let mut simplified_polygons: Vec<Polygon> = Vec::new();
    for (i, polygon) in polygons.iter().enumerate() {
        let simplified_polygon = polygon.simplify(&args.simplify_tolerance);

        if args.verbose {
            println!("Polygon {} simplified: {} -> {} points",
                     i,
                     polygon.exterior().points().count(),
                     simplified_polygon.exterior().points().count()
            );
        }

        if !args.skip_intermediates {
            let result_img = simplified_polygon.draw(width, height);
            let polygon_path = args.output.join(format!("{}_simplified_polygon_{}.png", asset_name, i));
            result_img.save(&polygon_path)
                .map_err(|e| format!("Failed to save simplified polygon image: {}", e))?;
        }

        simplified_polygons.push(simplified_polygon);
    }

    // Smooth the polygons using Chaikin's algorithm
    let mut smooth_polygons: Vec<Polygon> = Vec::new();
    for (i, polygon) in simplified_polygons.iter().enumerate() {
        let smooth_polygon = polygon.chaikin_smoothing(args.smooth_iterations);

        if args.verbose {
            println!("Polygon {} smoothed: {} -> {} points",
                     i,
                     polygon.exterior().points().count(),
                     smooth_polygon.exterior().points().count()
            );
        }

        if !args.skip_intermediates {
            let result_img = smooth_polygon.draw(width, height);
            let polygon_path = args.output.join(format!("{}_smooth_polygon_{}.png", asset_name, i));
            result_img.save(&polygon_path)
                .map_err(|e| format!("Failed to save smooth polygon image: {}", e))?;
        }

        smooth_polygons.push(smooth_polygon);
    }

    // Create the meshes from the polygons
    for (i, polygon) in smooth_polygons.iter().enumerate() {
        // Create 2D mesh
        let mesh2d = polygon.mesh2d()
            .map_err(|e| format!("Failed to create 2D mesh for polygon {}: {}", i, e))?;

        let mesh2d_path = args.output.join(format!("{}_{}.2d.obj", asset_name, i));
        mesh2d.export_obj(mesh2d_path.as_path())
            .map_err(|e| format!("Failed to export 2D mesh: {}", e))?;

        // Create 3D mesh
        let mesh3d = mesh2d.extrude(args.extrude_height, width as f64, height as f64);
        let mesh_path = args.output.join(format!("{}_{}.obj", asset_name, i));
        let material_path = args.output.join(format!("{}_{}.mtl", asset_name, i));

        mesh3d.export_obj(
            mesh_path.as_path(),
            material_path.as_path(),
            &texture_filename,
            &args.back_texture,
            &args.side_texture
        ).map_err(|e| format!("Failed to export 3D mesh: {}", e))?;

        if args.verbose {
            println!("Exported mesh {} to: {}", i, mesh_path.display());
        }
    }

    println!("Successfully generated {} meshes in: {}", smooth_polygons.len(), args.output.display());
    Ok(())
}

fn generate_binary_mask(image: &DynamicImage, method: &MaskMethod, threshold: u8) -> BinaryImage {
    match method {
        MaskMethod::Luminance => {
            // Convert to grayscale and threshold
            let gray = image.to_luma8();
            let binary_data: Vec<u8> = gray.pixels()
                .map(|pixel| if pixel.0[0] > threshold { 255 } else { 0 })
                .collect();
            BinaryImage::from_raw(gray.width(), gray.height(), &binary_data)
        },
        MaskMethod::Alpha => {
            let rgba = image.to_rgba8();
            let binary_data: Vec<u8> = rgba.pixels()
                .map(|pixel| if pixel.0[3] > threshold { 255 } else { 0 }) // Alpha channel
                .collect();
            BinaryImage::from_raw(rgba.width(), rgba.height(), &binary_data)
        },
        MaskMethod::Red => {
            let rgb = image.to_rgb8();
            let binary_data: Vec<u8> = rgb.pixels()
                .map(|pixel| if pixel.0[0] > threshold { 255 } else { 0 }) // Red channel
                .collect();
            BinaryImage::from_raw(rgb.width(), rgb.height(), &binary_data)
        },
        MaskMethod::Green => {
            let rgb = image.to_rgb8();
            let binary_data: Vec<u8> = rgb.pixels()
                .map(|pixel| if pixel.0[1] > threshold { 255 } else { 0 }) // Green channel
                .collect();
            BinaryImage::from_raw(rgb.width(), rgb.height(), &binary_data)
        },
        MaskMethod::Blue => {
            let rgb = image.to_rgb8();
            let binary_data: Vec<u8> = rgb.pixels()
                .map(|pixel| if pixel.0[2] > threshold { 255 } else { 0 }) // Blue channel
                .collect();
            BinaryImage::from_raw(rgb.width(), rgb.height(), &binary_data)
        },
    }
}
