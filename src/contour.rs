use crate::binary_image::BinaryImage;
use geo::{Polygon, LineString, Coord, Contains};

const O_VERTEX_WITH_BORDER: [(i8, i8); 7] = [(-1, 0), (0, 0), (-1, -1), (0, 0), (0, -1), (0, 0), (0, 0)]; // Bottom left coordinates with a border
const H_VERTEX_WITH_BORDER: [(i8, i8); 7] = [(0, 0), (0, 0), (-1, 0), (0, 0), (-1, -1), (0, 0), (0, -1)]; // Bottom right coordinates with a border
const O_VALUE_FOR_SIGNED:   [i8; 7]       = [1, 0, 2, 0, 4, 0, 8];     // Value to add into an array of contours (using signed integers)
const H_VALUE_FOR_SIGNED:   [i8; 7]       = [-4, 0, -8, 0, -1, 0, -2]; // (idem)
const MN: [(i8, i8); 8] = [(0, -1), (1, -1), (1, 0), (1, 1), (0, 1), (-1, 1), (-1, 0), (-1, -1)]; // Moore neighborhood

impl BinaryImage {

    pub fn trace_polygons(&self) -> Vec<Polygon> {
        let width = self.width() as usize;
        let height = self.height() as usize;
        let mut contours = vec![vec![0i8; width + 2]; height + 2];
        let mut polygons = Vec::new();
        let mut inner_rings = Vec::new();

        // Initialize contours array with proper values
        for y in 0..height {
            for x in 0..width {
                let pixel = self.get_pixel(x as u32, y as u32);
                contours[y + 1][x + 1] = if *pixel { 1 } else { -1 };
            }
        }

        let mut ol;
        let mut hl;

        for y in 1..=height {
            ol = 0;
            hl = 0;
            for x in 1..=width {
                // Check for exterior boundary (foreground pixel with background to the left)
                if ol == hl && contours[y][x] == 1 {
                    // Verify this is actually a boundary pixel by checking if there's a background pixel adjacent
                    let is_boundary = contours[y][x-1] <= 0 || contours[y-1][x] <= 0 ||
                        contours[y+1][x] <= 0 || contours[y][x+1] <= 0;

                    if is_boundary {
                        let ring = self.trace_polygon(
                            true,
                            x,
                            y,
                            [2, 3, 4, 5, 6, 7, 0, 1],
                            2,
                            (7, 1, 0),
                            O_VERTEX_WITH_BORDER,
                            O_VALUE_FOR_SIGNED,
                            &mut contours,
                        );
                        polygons.push(Polygon::new(ring, vec![]));
                    }
                }
                // Check for interior boundary (background pixel with foreground to the left, indicating a hole)
                else if ol > hl && contours[y][x] == -1 {
                    // Verify this is actually an interior boundary by checking adjacent pixels
                    let is_interior_boundary = contours[y][x-1] > 0 || contours[y-1][x] > 0 ||
                        contours[y+1][x] > 0 || contours[y][x+1] > 0;

                    if is_interior_boundary {
                        let ring = self.trace_polygon(
                            false,
                            x,
                            y,
                            [4, 5, 6, 7, 0, 1, 2, 3],
                            -2,
                            (1, 7, 6),
                            H_VERTEX_WITH_BORDER,
                            H_VALUE_FOR_SIGNED,
                            &mut contours,
                        );
                        inner_rings.push(ring);
                    }
                }

                // Update ol and hl counters based on contour values
                let abs_val = contours[y][x].abs();
                match abs_val {
                    2 | 4 | 10 | 12 => {
                        if contours[y][x] > 0 {
                            ol += 1
                        } else {
                            hl += 1
                        }
                    }
                    5 | 7 | 13 | 15 => {
                        if contours[y][x] > 0 {
                            ol -= 1
                        } else {
                            hl -= 1
                        }
                    }
                    _ => (),
                }
            }
        }

        // Assign interior rings to their containing polygons
        for inner_ring in inner_rings {
            // Find the smallest polygon that contains this inner ring
            let mut best_polygon_idx = None;
            let mut best_area = f64::INFINITY;

            for (idx, polygon) in polygons.iter().enumerate() {
                if self.polygon_contains_ring(polygon.exterior(), &inner_ring) {
                    let area = self.calculate_polygon_area(polygon.exterior());
                    if area < best_area {
                        best_area = area;
                        best_polygon_idx = Some(idx);
                    }
                }
            }

            if let Some(idx) = best_polygon_idx {
                polygons[idx].interiors_push(inner_ring);
            }
        }

        polygons
    }

    fn trace_polygon(
        &self,
        outline: bool,
        cursor_x: usize,
        cursor_y: usize,
        mut o: [usize; 8],
        rot: i8,
        viv: (usize, usize, usize),
        vertex: [(i8, i8); 7],
        value: [i8; 7],
        contours: &mut Vec<Vec<i8>>,
    ) -> LineString {
        let mut tracer_x = cursor_x;
        let mut tracer_y = cursor_y;
        let mut vertices_nbr: usize = 1;
        let mut ring = Vec::<Coord>::new();

        ring.push(Coord {
            x: (tracer_x as f64) + (vertex[o[0]].0 as f64),
            y: (tracer_y as f64) + (vertex[o[0]].1 as f64),
        });

        let mut neighbors: [i8; 8];
        let mut rn: u8;

        loop {
            // Safely access neighbors with bounds checking
            neighbors = [
                if tracer_y > 0 { contours[tracer_y - 1][tracer_x] } else { 0 },                                    // 0
                if tracer_y > 0 && tracer_x < contours[0].len() - 1 { contours[tracer_y - 1][tracer_x + 1] } else { 0 }, // 1
                if tracer_x < contours[0].len() - 1 { contours[tracer_y][tracer_x + 1] } else { 0 },               // 2
                if tracer_y < contours.len() - 1 && tracer_x < contours[0].len() - 1 { contours[tracer_y + 1][tracer_x + 1] } else { 0 }, // 3
                if tracer_y < contours.len() - 1 { contours[tracer_y + 1][tracer_x] } else { 0 },                  // 4
                if tracer_y < contours.len() - 1 && tracer_x > 0 { contours[tracer_y + 1][tracer_x - 1] } else { 0 }, // 5
                if tracer_x > 0 { contours[tracer_y][tracer_x - 1] } else { 0 },                                  // 6
                if tracer_y > 0 && tracer_x > 0 { contours[tracer_y - 1][tracer_x - 1] } else { 0 },             // 7
            ];

            rn = if outline {
                if neighbors[o[7]] > 0 && neighbors[o[0]] > 0 {
                    1
                } else if neighbors[o[0]] > 0 {
                    2
                } else if neighbors[o[1]] > 0 && neighbors[o[2]] > 0 {
                    3
                } else {
                    0
                }
            } else if neighbors[o[1]] < 0 && neighbors[o[0]] < 0 {
                1
            } else if neighbors[o[0]] < 0 {
                2
            } else if neighbors[o[7]] < 0 && neighbors[o[6]] < 0 {
                3
            } else {
                0
            };

            match rn {
                1 => {
                    contours[tracer_y][tracer_x] += value[o[0]];
                    tracer_x = tracer_x.wrapping_add(MN[o[viv.0]].0 as usize);
                    tracer_y = tracer_y.wrapping_add(MN[o[viv.0]].1 as usize);
                    // Rotate 90 degrees, counterclockwise for the outlines (rot = 2) or clockwise for the holes (rot = -2)
                    o.rotate_right(rot.rem_euclid(8) as usize);
                    vertices_nbr += 1;
                }
                2 => {
                    contours[tracer_y][tracer_x] += value[o[0]];
                    tracer_x = tracer_x.wrapping_add(MN[o[0]].0 as usize);
                    tracer_y = tracer_y.wrapping_add(MN[o[0]].1 as usize);
                }
                3 => {
                    contours[tracer_y][tracer_x] += value[o[0]];
                    // Rotate 90 degrees, clockwise for the outlines (rot = 2) or counterclockwise for the holes (rot = -2)
                    o.rotate_left(rot.rem_euclid(8) as usize);
                    contours[tracer_y][tracer_x] += value[o[0]];
                    vertices_nbr += 1;
                    ring.push(Coord {
                        x: (tracer_x as f64) + (vertex[o[0]].0 as f64),
                        y: (tracer_y as f64) + (vertex[o[0]].1 as f64),
                    });
                    o.rotate_right(rot.rem_euclid(8) as usize);
                    tracer_x = tracer_x.wrapping_add(MN[o[viv.1]].0 as usize);
                    tracer_y = tracer_y.wrapping_add(MN[o[viv.1]].1 as usize);
                    vertices_nbr += 1;
                }
                _ => {
                    contours[tracer_y][tracer_x] += value[o[0]];
                    o.rotate_left(rot.rem_euclid(8) as usize);
                    vertices_nbr += 1;
                },
            }

            // Check if we've returned to the starting position
            if tracer_x == cursor_x && tracer_y == cursor_y && vertices_nbr > 2 {
                break;
            }

            if rn != 2 {
                ring.push(Coord {
                    x: (tracer_x as f64) + (vertex[o[0]].0 as f64),
                    y: (tracer_y as f64) + (vertex[o[0]].1 as f64),
                });
            }
        }

        // Final cleanup loop
        loop {
            contours[tracer_y][tracer_x] += value[o[0]];
            if o[0] == viv.2 {
                break;
            }
            o.rotate_left(rot.rem_euclid(8) as usize);
            vertices_nbr += 1;
            ring.push(Coord {
                x: (tracer_x as f64) + (vertex[o[0]].0 as f64),
                y: (tracer_y as f64) + (vertex[o[0]].1 as f64),
            });
        }

        LineString::from(ring)
    }

    // Helper function to check if a polygon contains a ring
    fn polygon_contains_ring(&self, exterior: &LineString, inner_ring: &LineString) -> bool {
        // Simple point-in-polygon test using the first point of the inner ring
        if let Some(test_point) = inner_ring.coords().next() {
            self.point_in_polygon(test_point, exterior)
        } else {
            false
        }
    }

    // Helper function for point-in-polygon test using ray casting
    fn point_in_polygon(&self, point: &Coord, polygon: &LineString) -> bool {
        let mut inside = false;
        let coords: Vec<&Coord> = polygon.coords().collect();
        let n = coords.len();

        if n < 3 {
            return false;
        }

        let mut j = n - 1;
        for i in 0..n {
            let xi = coords[i].x;
            let yi = coords[i].y;
            let xj = coords[j].x;
            let yj = coords[j].y;

            if ((yi > point.y) != (yj > point.y)) &&
                (point.x < (xj - xi) * (point.y - yi) / (yj - yi) + xi) {
                inside = !inside;
            }
            j = i;
        }
        inside
    }

    // Helper function to calculate polygon area (for finding the smallest containing polygon)
    fn calculate_polygon_area(&self, polygon: &LineString) -> f64 {
        let coords: Vec<&Coord> = polygon.coords().collect();
        let n = coords.len();

        if n < 3 {
            return 0.0;
        }

        let mut area = 0.0;
        let mut j = n - 1;

        for i in 0..n {
            area += (coords[j].x + coords[i].x) * (coords[j].y - coords[i].y);
            j = i;
        }

        area.abs() / 2.0
    }
}
