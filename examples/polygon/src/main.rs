use std::fs;
use std::path::Path;
use image::{RgbaImage, Rgba, Luma, ImageBuffer, GenericImageView};
use imageproc::drawing::draw_polygon_mut;
use geo::{Polygon};
use imageproc::point::Point;
use mimesis::BinaryImage;

/// Convert a geo::Polygon into Vec<Point<i32>> suitable for imageproc
fn polygon_to_points(polygon: &Polygon<i32>) -> Vec<Point<i32>> {
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
fn draw_polygons(polygons: &[Polygon<i32>], width: u32, height: u32) -> RgbaImage {
    let mut img = RgbaImage::new(width, height);

    let color = Rgba([255, 0, 0, 255]); // Red outline

    for polygon in polygons {
        let points = polygon_to_points(polygon);
        draw_polygon_mut(&mut img, &points, color);
    }

    img
}


fn main() {
    let input_path = Path::new("assets/red.png");
    let image = image::open(input_path).expect("Failed to open image");

    let (width, height) = image.dimensions();
    let binary = BinaryImage::from(image);

    let visual = ImageBuffer::from_fn(binary.width(), binary.height(), |x, y| {
        let pixel = binary.get_pixel(x, y);
        if *pixel {
            Luma([255u8])
        } else {
            Luma([0u8])
        }
    });

    let asset_name = input_path.file_stem().unwrap().to_string_lossy();

    let out_dir = Path::new("out");
    fs::create_dir_all(out_dir).expect("Failed to create output folder");
    let mask_path = out_dir.join(format!("{}_mask.png", asset_name));
    visual.save(mask_path).expect("Failed to save binary image");

    let polygons: Vec<Polygon<i32>> = binary.trace_polygons();

    for (i, poly) in polygons.iter().enumerate() {
        println!("Polygon {}: {:?}", i, poly);
    }

    let result_img = draw_polygons(&polygons, width, height);
    let polygon_path = out_dir.join(format!("{}_polygon.png", asset_name));
    result_img.save(polygon_path).expect("Failed to save output");
}