use mimesis::mesh::PolygonMesh;
use std::fs;
use std::path::Path;
use image::{RgbaImage, Rgba, Luma, ImageBuffer, GenericImageView};
use imageproc::drawing::draw_polygon_mut;
use geo::{ChaikinSmoothing, Polygon, Simplify};
use imageproc::point::Point;
use mimesis::BinaryImage;

/// Convert a geo::Polygon into Vec<Point<i32>> suitable for imageproc
fn polygon_to_points(polygon: &Polygon) -> Vec<Point<i32>> {
    let polygon: Vec<Point<i32>> = polygon.exterior()
        .points()
        .map(|p| Point::new(p.x() as i32, p.y() as i32))
        .collect();

    if polygon.len() >= 2 && polygon.first() == polygon.last() {
        polygon[..polygon.len() - 1].to_vec()
    } else {
        polygon
    }
}

/// Draw polygons onto an image buffer
fn draw_polygons(polygons: &[Polygon], width: u32, height: u32) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);

    let color = Rgba([255, 0, 0, 255]); // Red outline

    for polygon in polygons {
        let points = polygon_to_points(polygon);
        draw_polygon_mut(&mut img, &points, color);
    }

    img
}


fn main() {
    let input_path = Path::new("assets/cow.png");
    let asset_name = input_path.file_stem().unwrap().to_string_lossy();
    let image = image::open(input_path).expect("Failed to open image");

    let (width, height) = image.dimensions();

    // Create binary mask from image
    let binary = BinaryImage::from(image);
    let visual = ImageBuffer::from_fn(binary.width(), binary.height(), |x, y| {
        let pixel = binary.get_pixel(x, y);
        if *pixel {
            Luma([255u8])
        } else {
            Luma([0u8])
        }
    });


    let out_dir = Path::new("out");
    fs::create_dir_all(out_dir).expect("Failed to create output folder");
    let mask_path = out_dir.join(format!("{}_mask.png", asset_name));
    visual.save(mask_path).expect("Failed to save binary image");

    // Convert binary mask to polygons using Theo Pavlidis' contour tracing algorithm
    let polygons: Vec<Polygon> = binary.trace_polygons();
    let result_img = draw_polygons(&polygons, width, height);
    let polygon_path = out_dir.join(format!("{}_polygon.png", asset_name));
    result_img.save(polygon_path).expect("Failed to save output");

    // Simplify the polygons using Ramer–Douglas–Peucker algorithm
    let mut simplified_polygons: Vec<Polygon> = Vec::new();
    for polygon in polygons.iter() {
        let simplified_polygon = polygon.simplify(&10.0);
        println!("Polygon simplified {} -> {}", polygon.exterior().points().count(), simplified_polygon.exterior().points().count());
        simplified_polygons.push(simplified_polygon)
    }
    let simplified_result_img = draw_polygons(&simplified_polygons, width, height);
    let simplified_polygon_path = out_dir.join(format!("{}_simplified_polygon.png", asset_name));
    simplified_result_img.save(simplified_polygon_path).expect("Failed to save output");

    // Smooth the polygons using Chaikin's algorithm
    let mut smooth_polygons: Vec<Polygon> = Vec::new();
    for polygon in simplified_polygons.iter() {
        let smooth_polygon = polygon.chaikin_smoothing(1);
        println!("Polygon smoothed {} -> {}", polygon.exterior().points().count(), smooth_polygon.exterior().points().count());
        smooth_polygons.push(smooth_polygon)
    }
    let smooth_result_img = draw_polygons(&smooth_polygons, width, height);
    let smooth_polygon_path = out_dir.join(format!("{}_smooth_polygon.png", asset_name));
    smooth_result_img.save(smooth_polygon_path).expect("Failed to save output");
    
    // Create the meshes from the polygons
    for (i, polygon) in smooth_polygons.iter().enumerate() {
        let mesh_path = out_dir.join(format!("{}_{}.obj", asset_name, i));
        let mesh2d = polygon.mesh2d().expect("Failed to create mesh");
        let mesh3d = mesh2d.extrude(20.0);
        mesh3d.export_obj(mesh_path.as_path()).unwrap()
    }
}