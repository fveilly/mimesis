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
                if ol == hl && contours[y][x] == 1 {
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
                } else if ol > hl && contours[y][x] == -1 {
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

                match contours[y][x].abs() {
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

        for inner_ring in inner_rings {
            for polygon in polygons.iter_mut() {
                if polygon.exterior().contains(&inner_ring) {
                    polygon.interiors_push(inner_ring);
                    break;
                }
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
            neighbors = [
                contours[tracer_y - 1][tracer_x],     // 0
                contours[tracer_y - 1][tracer_x + 1], // 1
                contours[tracer_y][tracer_x + 1],     // 2
                contours[tracer_y + 1][tracer_x + 1], // 3
                contours[tracer_y + 1][tracer_x],     // 4
                contours[tracer_y + 1][tracer_x - 1], // 5
                contours[tracer_y][tracer_x - 1],     // 6
                contours[tracer_y - 1][tracer_x - 1], // 7
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
            if tracer_x == cursor_x && tracer_y == cursor_y && vertices_nbr > 2 {
                break;
            }

            if (rn != 2) {
                ring.push(Coord {
                    x: (tracer_x as f64) + (vertex[o[0]].0 as f64),
                    y: (tracer_y as f64) + (vertex[o[0]].1 as f64),
                });
            }
        }

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

}