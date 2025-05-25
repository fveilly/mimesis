# 🖼️ Mimesis

This Rust project transforms a 2D image into a 3D mesh by extracting its alpha-based silhouette and generating a triangulated, extruded mesh from it.

## ✨ Features

- Extracts binary mask from an image based on the **alpha channel**
- Detects **polygon contours** using **Theo Pavlidis' contour tracing algorithm**
- Smooths and simplifies the polygon
- Triangulates the polygon using the **Earcutr** algorithm
- Extrudes the 2D mesh into a **3D shape** with configurable depth
- Maps the original image onto the extruded mesh IV
- Exports the result to a wavefront obj file

cargo run -- --verbose --texture assets\cow.jpg --mask assets\cow_mask.png
