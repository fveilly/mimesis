use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use earcutr::{earcut, Error};
use geo::{CoordsIter, Polygon};

#[derive(Debug, Clone, PartialEq)]
pub struct Mesh3D {
    pub vertices: Vec<[f64; 3]>,
    pub indices: Vec<[usize; 3]>,
}

impl Mesh3D {
    pub fn export_obj(&self, path: &Path) -> std::io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        // Write vertices
        for [x, y, z] in &self.vertices {
            writeln!(writer, "v {} {} {}", x, y, z)?;
        }

        // Write faces (OBJ indices are 1-based)
        for [i0, i1, i2] in &self.indices {
            writeln!(writer, "f {} {} {}", i0 + 1, i1 + 1, i2 + 1)?;
        }

        Ok(())
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
        let mut vertices = Vec::with_capacity(n * 2);
        let mut indices = Vec::new();

        // Create bottom (z = 0) and top (z = depth) vertices
        for [x, y] in &self.vertices {
            vertices.push([*x, *y, 0.0]);      // Bottom
        }
        for [x, y] in &self.vertices {
            vertices.push([*x, *y, depth]);    // Top
        }

        // Front and back faces (top and bottom)
        for triangle in self.indices.chunks(3) {
            let i0 = triangle[0];
            let i1 = triangle[1];
            let i2 = triangle[2];

            // Bottom face (original winding)
            indices.push([i0, i1, i2]);

            // Top face (reverse winding)
            indices.push([i2 + n, i1 + n, i0 + n]);
        }

        // Create side walls
        // Each edge becomes 2 triangles forming a quad
        let mut edge_set = std::collections::HashSet::new();

        for triangle in self.indices.chunks(3) {
            for e in 0..3 {
                let i0 = triangle[e];
                let i1 = triangle[(e + 1) % 3];

                let edge = if i0 < i1 { (i0, i1) } else { (i1, i0) };
                if edge_set.insert(edge) {
                    let (b0, b1) = edge;
                    let t0 = b0 + n;
                    let t1 = b1 + n;

                    // Two triangles forming a quad
                    indices.push([b0, b1, t1]);
                    indices.push([b0, t1, t0]);
                }
            }
        }

        Mesh3D { vertices, indices }
    }
}

impl Mesh3D {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }
}

pub trait PolygonMesh {
    fn mesh2d(&self) -> Result<Mesh2D, Error>;
}

impl PolygonMesh for Polygon {
    fn mesh2d(&self) -> Result<Mesh2D, Error> {
        let mut vertices: Vec<[f64; 2]> = Vec::new();
        let mut coords: Vec<f64> = Vec::new();
        let mut holes: Vec<usize> = Vec::new();

        for coord in self.exterior().points() {
            coords.push(coord.x());
            coords.push(coord.y());
            vertices.push([coord.x(), coord.y()]);
        }

        let mut offset = self.exterior().coords_count();

        for hole in self.interiors() {
            holes.push(coords.len() / 2);
            for coord in hole.points() {
                coords.push(coord.x());
                coords.push(coord.y());
                vertices.push([coord.x(), coord.y()]);
            }
            offset += hole.coords_count();
        }

        let indices = earcut(&coords, &holes, 2)?;

        Ok(Mesh2D { vertices, indices })
    }
}