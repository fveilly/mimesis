use geo::{Polygon};
use image::{Rgba, RgbaImage};
use imageproc::drawing::{draw_polygon_mut};
use imageproc::point::Point;

pub trait DrawMesh {
    fn draw(&self, width: u32, height: u32) -> RgbaImage;
}

/// Convert a geo::Polygon LineString into Vec<Point<i32>> suitable for imageproc
fn linestring_to_points(linestring: &geo::LineString) -> Vec<Point<i32>> {
    let points: Vec<Point<i32>> = linestring
        .points()
        .map(|p| Point::new(p.x() as i32, p.y() as i32))
        .collect();

    // Remove duplicate last point if it matches the first (closed polygon)
    if points.len() >= 2 && points.first() == points.last() {
        points[..points.len() - 1].to_vec()
    } else {
        points
    }
}

impl DrawMesh for Polygon {
    fn draw(&self, width: u32, height: u32) -> RgbaImage {
        let mut img = RgbaImage::new(width, height);

        let exterior_color = Rgba([255, 0, 0, 255]);
        let interior_color = Rgba([0, 0, 255, 255]);

        // Draw exterior ring outline
        let exterior_points = linestring_to_points(self.exterior());
        draw_polygon_mut(&mut img, &exterior_points, exterior_color);

        // Draw interior ring outlines
        for interior in self.interiors() {
            let interior_points = linestring_to_points(interior);
            draw_polygon_mut(&mut img, &interior_points, interior_color);
        }

        img
    }
}