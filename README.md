# 🖼️ Mimesis

Generate 3D meshes from images using contour tracing and polygon extrusion.

## ✨ Features

- Extracts binary mask from an image based on the **alpha channel**
- Detects **polygon contours** using **Theo Pavlidis' contour tracing algorithm**
- Smooths and simplifies the polygon
- Triangulates the polygon using the **Earcutr** algorithm
- Extrudes the 2D mesh into a **3D shape** with configurable depth
- Maps the original image onto the extruded mesh IV
- Exports the result to a wavefront obj file

| Step | Description                      | Image                                                                             |
| ---- |----------------------------------|-----------------------------------------------------------------------------------|
| 1️⃣  | **Original Image**               | ![Original](https://github.com/fveilly/mimesis/blob/main/bin/assets/girl.png)     |
| 2️⃣  | **Binary Mask**                  | ![Mask](https://github.com/fveilly/mimesis/blob/main/bin/assets/girl_mask.png)    |
| 3️⃣  | **Polygon Contour** (smoothed)   | ![Contour](https://github.com/fveilly/mimesis/blob/main/docs/girl_polygon.png)    |
| 4️⃣  | **Extruded 3D Mesh**             | ![3D](https://github.com/fveilly/mimesis/blob/main/docs/girl_3d.png)              |
| 5️⃣  | **Vertex View**                  | ![Wireframe](https://github.com/fveilly/mimesis/blob/main/docs/girl_3d_wired.png) |

## Installation

```bash
cargo build --release
```

## Usage

### Basic Usage

```bash
# Process a single image
./mimesis -i texture.png -o output/

# Process with custom mask
./mimesis -i texture.png -m mask.png -o output/

# Batch process directory
./mimesis -i images/ -o output/
```

### Configuration File

Generate a default configuration file:

```bash
./mimesis --generate-config -c config.json
```

Use configuration file:

```bash
./mimesis -c config.json
```

## Command Line Options

### Input/Output
- `-i, --input <PATH>` - Input image file or directory
- `-m, --mask <PATH>` - Optional binary mask image
- `-o, --output <PATH>` - Output directory
- `-c, --config <PATH>` - Configuration file path

### Processing Parameters
- `--onnx-background-removal` - Enable ONNX background removal
- `--onnx-model-path` - Path to the ONXX model
- `--simplify-tolerance <FLOAT>` - Polygon simplification tolerance (default: 10.0)
- `--smooth-iterations <INT>` - Number of smoothing iterations (default: 1)
- `--extrude-height <FLOAT>` - 3D extrusion height (default: 20.0)
- `--min-polygon-dimension <INT>` - Minimum polygon size in pixels (default: 0)
- `--threshold <INT>` - Binary mask threshold 0-255 (default: 128)
- `--mask-method <METHOD>` - Mask generation method: `alpha`, `luminance`, `red`, `green`, `blue` (default: alpha)

### Batch Processing
- `--include-patterns <PATTERNS>` - File patterns to include (e.g., "*.png,*.jpg")
- `--exclude-patterns <PATTERNS>` - File patterns to exclude
- `--workers <INT>` - Number of parallel workers (default: 1)
- `--continue-on-error` - Continue processing if some files fail

### Output Options
- `--side-texture <PATH>` - Custom side texture file
- `--back-texture <PATH>` - Custom back texture file
- `--skip-intermediates` - Skip saving intermediate files

### Other
- `--generate-config` - Generate default config file and exit
- `-v, --verbose` - Verbose output
- `--benchmark` - Benchmark output

## Mask Generation Methods

When no mask is provided, the tool can auto-generate binary masks using:

- **Alpha** - Uses alpha channel transparency (default)
- **Luminance** - Uses brightness/luminance values
- **Red/Green/Blue** - Uses individual color channels

## Output Structure

For each processed image, the tool generates:

```
output/
├── textures/
│   ├── image_name.png      # Front texture
│   ├── side.png            # Side texture (if provided)
│   └── back.png            # Back texture (if provided)
├── image_name_0.obj        # 3D mesh file
├── image_name_0.mtl        # Material file
```

## Batch Processing

When processing directories:

1. All matching files are found using include/exclude patterns
2. For each image, the tool looks for a corresponding mask file with `_mask` suffix
3. If no mask is found, one is auto-generated
4. Files with `_mask` in the name are automatically excluded from processing

Example batch structure:
```
input/
├── sprite1.png
├── sprite1_mask.png    # Optional custom mask
├── sprite2.png
└── character.jpg
```

## Examples

### Generate mesh with custom parameters

```bash
./mimesis \
  -i character.png \
  -o models/ \
  --extrude-height 30.0 \
  --simplify-tolerance 5.0 \
  --smooth-iterations 2 \
  --threshold 200 \
  --mask-method luminance
```

### Batch process with configuration

```bash
# Generate config template
./mimesis --generate-config -c batch_config.yaml

# Edit config file, then run
./mimesis -c batch_config.yaml -i sprites/ -o output/
```

### Process with custom textures

```bash
./mimesis \
  -i logo.png \
  -o output/ \
  --side-texture wood_texture.jpg \
  --back-texture metal_texture.jpg
```

## Background removal

This feature allows you to run background removal on images using the RMBG-1.4 model.

### Model

- Download the ONNX model here:  
  [`RMBG-1.4.onnx`](https://huggingface.co/briaai/RMBG-1.4/blob/main/onnx/model.onnx)

### Usage

1. **Set the ONNX Runtime library path** using the ORT_LIB_LOCATION environment variable.

Refer to this [`guide`](https://ort.pyke.io/setup/linking) for details.

Example (Windows):
```bash
set ORT_LIB_LOCATION=C:\path\to\onnxruntime.dll
```

2. **Enable the feature** in your Cargo run command:

```bash
cargo run --features background-remover
```

## Supported Formats

### Input Images
- PNG, JPEG, BMP, TIFF, TGA
- RGB and RGBA formats supported
- Alpha channel used for mask generation when available

### Output Formats
- OBJ (Wavefront) mesh files
- MTL (Material) files
- PNG textures and visualizations

### Configuration Files
- JSON (.json)
- TOML (.toml)

## Performance Tips

1. **Simplification**: Higher `simplify_tolerance` values create simpler meshes
2. **Smoothing**: More iterations create smoother curves but increase processing time
3. **Minimum polygon size**: Filter out small noise polygons
4. **Batch processing**: Use `--workers` for parallel processing (TO BE IMPLEMENTED)
5. **Skip intermediates**: Use `--skip-intermediates` to save disk space