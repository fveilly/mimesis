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
    pub uvs: Option<Vec<[f64; 2]>>,
    pub faces: Vec<MeshGroup>,
}

impl Mesh3D {
    pub fn export_obj(&self, path: &Path) -> std::io::Result<()> {

        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        writeln!(writer, "o Mesh3D")?;

        // Write vertices
        for [x, y, z] in &self.vertices {
            writeln!(writer, "v {} {} {}", x, y, z)?;
        }

        // Optionally write texture coordinates if present
        if let Some(uvs) = &self.uvs {
            for [u, v] in uvs {
                writeln!(writer, "vt {} {}", u, v)?;
            }
        }

        // Write face groups
        for group in &self.faces {
            writeln!(writer, "g {}", group.name)?;
            for [i0, i1, i2] in &group.indices {
                // Only use vertex index (v), not vt or vn
                writeln!(writer, "f {} {} {}", i0 + 1, i1 + 1, i2 + 1)?;
            }
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
        let mut front_indices = Vec::new();
        let mut back_indices = Vec::new();
        let mut side_indices = Vec::new();

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

            // Front face = original winding
            front_indices.push([i0, i1, i2]);

            // Back face = reversed winding on top vertices
            back_indices.push([i2 + n, i1 + n, i0 + n]);
        }

        // Count how many times each edge appears
        let mut edge_count: HashMap<(usize, usize), usize> = HashMap::new();
        for triangle in self.indices.chunks(3) {
            for e in 0..3 {
                let i0 = triangle[e];
                let i1 = triangle[(e + 1) % 3];
                let edge = if i0 < i1 { (i0, i1) } else { (i1, i0) };
                *edge_count.entry(edge).or_insert(0) += 1;
            }
        }

        // Only create sides for boundary edges (those used once)
        for (&(b0, b1), &count) in &edge_count {
            if count == 1 {
                let t0 = b0 + n;
                let t1 = b1 + n;

                side_indices.push([b0, b1, t1]);
                side_indices.push([b0, t1, t0]);
            }
        }

        Mesh3D {
            vertices,
            uvs: None,
            faces: vec![
                MeshGroup { indices: front_indices, name: "front" },
                MeshGroup { indices: back_indices, name: "back" },
                MeshGroup { indices: side_indices, name: "side" },
            ],
        }
    }
}

impl Mesh3D {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            uvs: None,
            faces: Vec::new(),
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