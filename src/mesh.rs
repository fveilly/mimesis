use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use earcutr::{earcut, Error};
use geo::{CoordsIter, Polygon};

#[derive(Debug, Clone)]
pub struct MeshGroup {
    pub indices: Vec<[usize; 3]>,
    pub name: &'static str,
}

#[derive(Debug, Clone)]
pub struct Mesh3D {
    pub vertices: Vec<[f64; 3]>,
    pub uvs: Vec<[f64; 2]>,
    pub faces: Vec<MeshGroup>,
}

impl Mesh3D {
    pub fn export_obj(&self, obj_path: &Path, mtl_path: &Path, front_texture: &str, back_texture: &str, side_texture: &str) -> std::io::Result<()> {
        self.export_mtl(mtl_path, front_texture, back_texture, side_texture)?;

        let file = File::create(obj_path)?;
        let mut writer = BufWriter::new(file);

        // Add MTL reference
        let mtl_filename = mtl_path.file_name().unwrap().to_string_lossy();
        writeln!(writer, "mtllib {}", mtl_filename)?;
        writeln!(writer, "o Mesh3D")?;

        // Write vertices
        for [x, y, z] in &self.vertices {
            writeln!(writer, "v {} {} {}", x, y, z)?;
        }

        // Write texture coordinates
        for [u, v] in &self.uvs {
            writeln!(writer, "vt {} {}", u, v)?;
        }

        // Write face groups
        for group in &self.faces {
            writeln!(writer, "usemtl {}", group.name)?;
            writeln!(writer, "g {}", group.name)?;
            for [i0, i1, i2] in &group.indices {
                writeln!(
                    writer,
                    "f {0}/{0} {1}/{1} {2}/{2}",
                    i0 + 1,
                    i1 + 1,
                    i2 + 1
                )?;
            }
        }

        Ok(())
    }

    fn export_mtl(&self, path: &Path, front_texture: &str, back_texture: &str, side_texture: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;

        // Front material
        writeln!(file, "newmtl front")?;
        writeln!(file, "Ka 1.0 1.0 1.0")?;
        writeln!(file, "Kd 1.0 1.0 1.0")?;
        writeln!(file, "Ks 0.0 0.0 0.0")?;
        writeln!(file, "d 1.0")?;
        writeln!(file, "Ns 10.0")?;
        writeln!(file, "illum 2")?;
        writeln!(file, "map_Kd {}", front_texture)?;

        // Back material
        writeln!(file, "\nnewmtl back")?;
        writeln!(file, "Ka 1.0 1.0 1.0")?;
        writeln!(file, "Kd 1.0 1.0 1.0")?;
        writeln!(file, "Ks 0.0 0.0 0.0")?;
        writeln!(file, "d 1.0")?;
        writeln!(file, "Ns 10.0")?;
        writeln!(file, "illum 2")?;
        writeln!(file, "map_Kd {}", back_texture)?;

        // Side material
        writeln!(file, "\nnewmtl side")?;
        writeln!(file, "Ka 1.0 1.0 1.0")?;
        writeln!(file, "Kd 1.0 1.0 1.0")?;
        writeln!(file, "Ks 0.0 0.0 0.0")?;
        writeln!(file, "d 1.0")?;
        writeln!(file, "Ns 10.0")?;
        writeln!(file, "illum 2")?;
        writeln!(file, "map_Kd {}", side_texture)?;

        Ok(())
    }
}

impl Mesh3D {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            uvs: Vec::new(),
            faces: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct Mesh2D {
    pub vertices: Vec<[f64; 2]>,
    pub indices: Vec<usize>,
}

impl Mesh2D {
    pub fn export_obj(&self, path: &Path) -> std::io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // Write vertex positions
        for [x, y] in &self.vertices {
            writeln!(writer, "v {} {} 0.0", x, y)?;
        }

        // Write triangle faces (OBJ is 1-based index)
        for face in self.indices.chunks(3) {
            writeln!(
                writer,
                "f {} {} {}",
                face[0] + 1,
                face[1] + 1,
                face[2] + 1
            )?;
        }

        Ok(())
    }

    pub fn extrude(&self, depth: f64) -> Mesh3D {
        let n = self.vertices.len();

        // We need separate vertices for different UV mappings
        // Structure: [bottom_vertices, top_vertices, side_vertices...]
        let mut vertices = Vec::new();
        let mut uvs = Vec::new();
        let mut front_indices = Vec::new();
        let mut back_indices = Vec::new();
        let mut side_indices = Vec::new();

        // Compute bounding box for UV normalization
        let (min_x, max_x) = self.vertices.iter()
            .map(|v| v[0])
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), x| (min.min(x), max.max(x)));
        let (min_y, max_y) = self.vertices.iter()
            .map(|v| v[1])
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), y| (min.min(y), max.max(y)));

        let dx = if (max_x - min_x).abs() < f64::EPSILON { 1.0 } else { max_x - min_x };
        let dy = if (max_y - min_y).abs() < f64::EPSILON { 1.0 } else { max_y - min_y };

        // Create bottom vertices (front face) - indices 0..n-1
        for [x, y] in &self.vertices {
            vertices.push([*x, *y, 0.0]);
            // UV mapping for front face (original image)
            uvs.push([(*x - min_x) / dx, (*y - min_y) / dy]);
        }

        // Create top vertices (back face) - indices n..2n-1
        for [x, y] in &self.vertices {
            vertices.push([*x, *y, depth]);
            // UV mapping for back face (can be same as front, or flipped)
            uvs.push([(*x - min_x) / dx, (*y - min_y) / dy]);
        }

        // Generate front and back faces
        for triangle in self.indices.chunks(3) {
            let i0 = triangle[0];
            let i1 = triangle[1];
            let i2 = triangle[2];

            // Front face (bottom, z=0) - keep original winding
            front_indices.push([i2 + n, i1 + n, i0 + n]);

            // Back face (top, z=depth) - reverse winding for correct normals
            back_indices.push([i0, i1, i2]);
        }

        // Find boundary edges for side faces
        let mut edge_count = std::collections::HashMap::new();
        for triangle in self.indices.chunks(3) {
            for e in 0..3 {
                let i0 = triangle[e];
                let i1 = triangle[(e + 1) % 3];
                let edge = if i0 < i1 { (i0, i1) } else { (i1, i0) };
                *edge_count.entry(edge).or_insert(0) += 1;
            }
        }

        // Collect boundary edges and sort them to form a continuous boundary
        let mut boundary_edges: Vec<(usize, usize)> = edge_count.iter()
            .filter(|&(_, &count)| count == 1)
            .map(|(&(i0, i1), _)| (i0, i1))
            .collect();

        // Sort boundary edges to form continuous loops
        boundary_edges.sort_by_key(|&(i0, i1)| i0.min(i1));

        // Calculate total perimeter for UV mapping
        let total_perimeter: f64 = boundary_edges.iter()
            .map(|&(i0, i1)| {
                let p0 = self.vertices[i0];
                let p1 = self.vertices[i1];
                ((p1[0] - p0[0]).powi(2) + (p1[1] - p0[1]).powi(2)).sqrt()
            })
            .sum();

        // Create side faces with proper UV mapping
        let mut current_u = 0.0;
        let vertex_offset = vertices.len(); // Starting index for new side vertices

        for &(i0, i1) in &boundary_edges {
            let p0 = self.vertices[i0];
            let p1 = self.vertices[i1];
            let edge_length = ((p1[0] - p0[0]).powi(2) + (p1[1] - p0[1]).powi(2)).sqrt();

            let u0 = current_u / total_perimeter;
            let u1 = (current_u + edge_length) / total_perimeter;
            current_u += edge_length;

            // Add 4 vertices for this side quad (to have unique UVs)
            let base_idx = vertices.len();

            // Bottom left (original i0)
            vertices.push([p0[0], p0[1], 0.0]);
            uvs.push([u0, 0.0]);

            // Bottom right (original i1)  
            vertices.push([p1[0], p1[1], 0.0]);
            uvs.push([u1, 0.0]);

            // Top right (extruded i1)
            vertices.push([p1[0], p1[1], depth]);
            uvs.push([u1, 1.0]);

            // Top left (extruded i0)
            vertices.push([p0[0], p0[1], depth]);
            uvs.push([u0, 1.0]);

            // Create two triangles for the side quad
            // Triangle 1: bottom-left, bottom-right, top-right
            side_indices.push([base_idx, base_idx + 1, base_idx + 2]);
            // Triangle 2: bottom-left, top-right, top-left
            side_indices.push([base_idx, base_idx + 2, base_idx + 3]);
        }

        Mesh3D {
            vertices,
            uvs,
            faces: vec![
                MeshGroup { indices: front_indices, name: "front" },
                MeshGroup { indices: back_indices, name: "back" },
                MeshGroup { indices: side_indices, name: "side" },
            ],
        }
    }
}

pub trait PolygonMesh {
    fn mesh2d(&self) -> Result<Mesh2D, Error>;
}


// Helper function to check if a ring is counter-clockwise
fn is_counter_clockwise(ring: &[[f64; 2]]) -> bool {
    if ring.len() < 3 {
        return true;
    }

    let mut signed_area = 0.0;
    for i in 0..ring.len() {
        let j = (i + 1) % ring.len();
        signed_area += (ring[j][0] - ring[i][0]) * (ring[j][1] + ring[i][1]);
    }
    signed_area < 0.0
}

// Helper function to reverse coordinate pairs in a flat coordinate array
fn reverse_ring(coords: &mut [f64]) {
    let pairs: Vec<[f64; 2]> = coords
        .chunks_exact(2)
        .map(|chunk| [chunk[0], chunk[1]])
        .collect();

    for (i, pair) in pairs.iter().rev().enumerate() {
        coords[i * 2] = pair[0];
        coords[i * 2 + 1] = pair[1];
    }
}

impl PolygonMesh for Polygon {
    fn mesh2d(&self) -> Result<Mesh2D, Error> {
        let mut vertices: Vec<[f64; 2]> = Vec::new();
        let mut coords: Vec<f64> = Vec::new();
        let mut holes: Vec<usize> = Vec::new();

        // Get bounding box for Y-flipping
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;

        // First pass: collect all coordinates to determine bounding box
        let exterior_points: Vec<_> = self.exterior().points().collect();
        let exterior_coords = if exterior_points.len() > 1 &&
            exterior_points[0].x() == exterior_points[exterior_points.len()-1].x() &&
            exterior_points[0].y() == exterior_points[exterior_points.len()-1].y() {
            &exterior_points[..exterior_points.len()-1]
        } else {
            &exterior_points[..]
        };

        // Update bounding box with exterior points
        for coord in exterior_coords {
            min_y = min_y.min(coord.y());
            max_y = max_y.max(coord.y());
        }

        // Update bounding box with hole points
        for hole in self.interiors() {
            let hole_points: Vec<_> = hole.points().collect();
            let hole_coords = if hole_points.len() > 1 &&
                hole_points[0].x() == hole_points[hole_points.len()-1].x() &&
                hole_points[0].y() == hole_points[hole_points.len()-1].y() {
                &hole_points[..hole_points.len()-1]
            } else {
                &hole_points[..]
            };

            for coord in hole_coords {
                min_y = min_y.min(coord.y());
                max_y = max_y.max(coord.y());
            }
        }

        let center_y = (min_y + max_y) * 0.5;

        // Process exterior ring with Y-flipping applied upfront
        for coord in exterior_coords {
            let x = coord.x();
            let y = 2.0 * center_y - coord.y(); // Flip Y coordinate here

            coords.push(x);
            coords.push(y);
            vertices.push([x, y]);
        }

        // Ensure exterior ring has correct winding order (counter-clockwise after Y-flip)
        let exterior_len = vertices.len();
        if !is_counter_clockwise(&vertices[0..exterior_len]) {
            // Reverse the exterior ring coordinates
            let exterior_coords_count = exterior_len * 2;
            reverse_ring(&mut coords[0..exterior_coords_count]);
            vertices[0..exterior_len].reverse();
        }

        // Process holes with Y-flipping applied upfront
        for hole in self.interiors() {
            holes.push(coords.len() / 2);

            let hole_points: Vec<_> = hole.points().collect();
            let hole_coords = if hole_points.len() > 1 &&
                hole_points[0].x() == hole_points[hole_points.len()-1].x() &&
                hole_points[0].y() == hole_points[hole_points.len()-1].y() {
                &hole_points[..hole_points.len()-1]
            } else {
                &hole_points[..]
            };

            let hole_start = vertices.len();
            for coord in hole_coords {
                let x = coord.x();
                let y = 2.0 * center_y - coord.y(); // Flip Y coordinate here

                coords.push(x);
                coords.push(y);
                vertices.push([x, y]);
            }

            // Ensure hole has correct winding order (clockwise after Y-flip)
            let hole_end = vertices.len();
            if is_counter_clockwise(&vertices[hole_start..hole_end]) {
                // Reverse the hole coordinates
                let hole_coords_start = hole_start * 2;
                let hole_coords_end = hole_end * 2;
                reverse_ring(&mut coords[hole_coords_start..hole_coords_end]);
                vertices[hole_start..hole_end].reverse();
            }
        }

        // Triangulate with correct winding orders
        let indices = earcut(&coords, &holes, 2)?;

        Ok(Mesh2D { vertices, indices })
    }
}