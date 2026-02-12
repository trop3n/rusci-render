use osci_core::shape::{Line, Shape, normalize_shapes_to};

/// Configuration for the image-to-shapes parser.
pub struct ImageConfig {
    /// Brightness threshold (0-255). Pixels above this are considered "on".
    pub threshold: u8,
    /// Row skip factor. Only every `stride`-th row is processed.
    pub stride: u32,
    /// If true, invert brightness before thresholding (dark pixels become "on").
    pub invert: bool,
}

impl Default for ImageConfig {
    fn default() -> Self {
        Self {
            threshold: 128,
            stride: 2,
            invert: false,
        }
    }
}

/// Parse image bytes into oscilloscope-renderable shapes using threshold scanning.
///
/// The algorithm converts the image to grayscale, then scans each row (stepping
/// by `config.stride`) to find continuous horizontal runs of pixels that exceed
/// the brightness threshold. Each run becomes a `Line` shape. The resulting
/// coordinates are normalized to the [-1, 1] range.
pub fn parse_image(data: &[u8], config: &ImageConfig) -> Result<Vec<Box<dyn Shape>>, String> {
    let img = image::load_from_memory(data)
        .map_err(|e| format!("failed to load image: {e}"))?;
    let gray = img.to_luma8();
    let width = gray.width();
    let height = gray.height();

    if width == 0 || height == 0 {
        return Ok(Vec::new());
    }

    let shapes = threshold_scan(&gray, width, height, config);

    let mut shapes = shapes;
    normalize_shapes_to(&mut shapes, width as f32, height as f32);

    Ok(shapes)
}

/// Scan a grayscale image row-by-row with the given config, producing horizontal
/// line segments for each continuous run of "on" pixels.
pub(crate) fn threshold_scan(
    gray: &image::GrayImage,
    width: u32,
    height: u32,
    config: &ImageConfig,
) -> Vec<Box<dyn Shape>> {
    let mut shapes: Vec<Box<dyn Shape>> = Vec::new();
    let stride = config.stride.max(1);

    let mut y = 0u32;
    while y < height {
        let mut in_segment = false;
        let mut start_x: u32 = 0;

        for x in 0..width {
            let pixel = gray.get_pixel(x, y).0[0];
            let is_on = if config.invert {
                pixel < config.threshold
            } else {
                pixel > config.threshold
            };

            if is_on {
                if !in_segment {
                    in_segment = true;
                    start_x = x;
                }
            } else if in_segment {
                // End of a segment
                shapes.push(Box::new(Line::new_2d(
                    start_x as f32,
                    y as f32,
                    x as f32,
                    y as f32,
                )));
                in_segment = false;
            }
        }

        // If the segment extends to the right edge
        if in_segment {
            shapes.push(Box::new(Line::new_2d(
                start_x as f32,
                y as f32,
                width as f32,
                y as f32,
            )));
        }

        y += stride;
    }

    shapes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ImageConfig::default();
        assert_eq!(config.threshold, 128);
        assert_eq!(config.stride, 2);
        assert!(!config.invert);
    }

    #[test]
    fn test_invalid_image_returns_error() {
        let result = parse_image(b"not an image", &ImageConfig::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_threshold_scan_simple() {
        // Create a 4x2 grayscale image:
        // Row 0: [255, 255, 0, 0]  -> one segment from x=0 to x=2
        // Row 1: [0, 0, 255, 255]  -> one segment from x=2 to x=4
        let pixels: Vec<u8> = vec![
            255, 255, 0, 0,
            0, 0, 255, 255,
        ];
        let gray = image::GrayImage::from_raw(4, 2, pixels).unwrap();

        let config = ImageConfig {
            threshold: 128,
            stride: 1,
            invert: false,
        };
        let shapes = threshold_scan(&gray, 4, 2, &config);

        // Should have 2 line segments
        assert_eq!(shapes.len(), 2);

        // First line: row 0, x from 0 to 2
        let start = shapes[0].next_vector(0.0);
        let end = shapes[0].next_vector(1.0);
        assert!((start.x - 0.0).abs() < 0.01);
        assert!((start.y - 0.0).abs() < 0.01);
        assert!((end.x - 2.0).abs() < 0.01);
        assert!((end.y - 0.0).abs() < 0.01);

        // Second line: row 1, x from 2 to 4
        let start = shapes[1].next_vector(0.0);
        let end = shapes[1].next_vector(1.0);
        assert!((start.x - 2.0).abs() < 0.01);
        assert!((start.y - 1.0).abs() < 0.01);
        assert!((end.x - 4.0).abs() < 0.01);
        assert!((end.y - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_threshold_scan_with_stride() {
        // 4x4 image, all white
        let pixels: Vec<u8> = vec![255; 16];
        let gray = image::GrayImage::from_raw(4, 4, pixels).unwrap();

        let config = ImageConfig {
            threshold: 128,
            stride: 2,
            invert: false,
        };
        let shapes = threshold_scan(&gray, 4, 4, &config);

        // With stride 2, only rows 0 and 2 should be scanned -> 2 segments
        assert_eq!(shapes.len(), 2);
    }

    #[test]
    fn test_threshold_scan_inverted() {
        // Row 0: [0, 0, 255, 255] -> inverted: dark pixels are "on" -> segment x=0..2
        let pixels: Vec<u8> = vec![0, 0, 255, 255];
        let gray = image::GrayImage::from_raw(4, 1, pixels).unwrap();

        let config = ImageConfig {
            threshold: 128,
            stride: 1,
            invert: true,
        };
        let shapes = threshold_scan(&gray, 4, 1, &config);

        assert_eq!(shapes.len(), 1);
        let start = shapes[0].next_vector(0.0);
        let end = shapes[0].next_vector(1.0);
        assert!((start.x - 0.0).abs() < 0.01);
        assert!((end.x - 2.0).abs() < 0.01);
    }
}
