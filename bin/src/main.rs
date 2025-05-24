use mimesis::draw::DrawMesh;
use mimesis::mesh::PolygonMesh;
use std::fs;
use std::path::Path;
use image::{Luma, ImageBuffer, GenericImageView};
use geo::{ChaikinSmoothing, Polygon, Simplify};
use mimesis::BinaryImage;

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
    for (i, polygon) in polygons.iter().enumerate() {
        println!("Polygon {}: exterior_points={} interior_rings={}", i, polygon.exterior().points().count(),
                 polygon.interiors().iter().count());
        for ip in polygon.interiors().iter() {
            println!("  interior_points={}", ip.points().count());       
        }
        let result_img = polygon.draw(width, height);
        let polygon_path = out_dir.join(format!("{}_polygon_{}.png", asset_name, i));
        result_img.save(polygon_path).expect("Failed to save output");
    }

    // Simplify the polygons using Ramer–Douglas–Peucker algorithm
    let mut simplified_polygons: Vec<Polygon> = Vec::new();
    for (i, polygon) in polygons.iter().enumerate() {
        let simplified_polygon = polygon.simplify(&10.0);
        println!("Polygon simplified {} -> {}", polygon.exterior().points().count(), simplified_polygon.exterior().points().count());

        let result_img = simplified_polygon.draw(width, height);
        let polygon_path = out_dir.join(format!("{}_simplified_polygon_{}.png", asset_name, i));
        result_img.save(polygon_path).expect("Failed to save output");

        simplified_polygons.push(simplified_polygon);
    }

    // Smooth the polygons using Chaikin's algorithm
    let mut smooth_polygons: Vec<Polygon> = Vec::new();
    for (i, polygon) in simplified_polygons.iter().enumerate() {
        let smooth_polygon = polygon.chaikin_smoothing(1);
        println!("Polygon smoothed {} -> {}", polygon.exterior().points().count(), smooth_polygon.exterior().points().count());

        let result_img = smooth_polygon.draw(width, height);
        let polygon_path = out_dir.join(format!("{}_smooth_polygon_{}.png", asset_name, i));
        result_img.save(polygon_path).expect("Failed to save output");

        smooth_polygons.push(smooth_polygon);
    }
    
    // Create the meshes from the polygons
    for (i, polygon) in smooth_polygons.iter().enumerate() {

        let mesh2d = polygon.mesh2d().expect("Failed to create mesh");
        mesh2d.export_obj(out_dir.join(format!("{}_{}.2d.obj", asset_name, i)).as_path()).expect("TODO: panic message");
        let mesh3d = mesh2d.extrude(20.0);
        let mesh_path = out_dir.join(format!("{}_{}.obj", asset_name, i));
        let material_path = out_dir.join(format!("{}_{}.mtl", asset_name, i));
        mesh3d.export_obj(mesh_path.as_path(), material_path.as_path(), "cow.png", "wood.jpg", "wood.jpg").unwrap();
    }
}