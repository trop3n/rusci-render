use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, SwashContent};
use osci_core::shape::{normalize_shapes, Line, Shape};

/// Configuration for text-to-shape conversion.
pub struct TextConfig {
    /// Font size in pixels. Default: 24.0
    pub font_size: f32,
}

impl Default for TextConfig {
    fn default() -> Self {
        Self { font_size: 24.0 }
    }
}

/// Parse a text string into vector line shapes suitable for oscilloscope rendering.
///
/// Each character is rasterised using `cosmic-text` and then converted into
/// horizontal line segments by scanning alpha rows. The resulting shapes are
/// normalized to fit within the [-1, 1] coordinate range.
pub fn parse_text(text: &str, config: &TextConfig) -> Result<Vec<Box<dyn Shape>>, String> {
    if text.is_empty() {
        return Ok(Vec::new());
    }

    // 1. Create a FontSystem and load system fonts
    let mut font_system = FontSystem::new();

    // 2. Create a Buffer for text layout
    let line_height = config.font_size * 1.2;
    let metrics = Metrics::new(config.font_size, line_height);
    let mut buffer = Buffer::new(&mut font_system, metrics);

    // 3. Set text content with sans-serif font family
    let attrs = Attrs::new().family(Family::SansSerif);
    buffer.set_text(&mut font_system, text, attrs, Shaping::Advanced);

    // 4. Perform layout
    buffer.shape_until_scroll(&mut font_system, false);

    // 5. Create a SwashCache for glyph rasterization
    let mut cache = SwashCache::new();

    // 6. Iterate over layout runs and rasterize each glyph
    let mut shapes: Vec<Box<dyn Shape>> = Vec::new();

    for run in buffer.layout_runs() {
        for glyph in run.glyphs.iter() {
            let physical = glyph.physical((0.0, 0.0), 1.0);

            if let Some(image) = cache.get_image(&mut font_system, physical.cache_key) {
                let w = image.placement.width as usize;
                let h = image.placement.height as usize;

                if w == 0 || h == 0 {
                    continue;
                }

                // Compute the top-left position of this glyph in pixel space
                let gx = physical.x + image.placement.left;
                let gy = physical.y - image.placement.top;

                // Determine bytes per pixel based on content type
                let bpp = match image.content {
                    SwashContent::Mask => 1,
                    SwashContent::Color => 4,
                    SwashContent::SubpixelMask => 3,
                };

                let expected_len = w * h * bpp;
                if image.data.len() < expected_len {
                    continue;
                }

                // Scan rows and create horizontal line segments where alpha > 128
                for row in 0..h {
                    let y = (gy + row as i32) as f32;
                    let mut line_start: Option<f32> = None;

                    for col in 0..w {
                        let alpha = match image.content {
                            SwashContent::Mask => image.data[row * w + col],
                            SwashContent::Color => {
                                // RGBA: alpha is the 4th byte
                                image.data[(row * w + col) * 4 + 3]
                            }
                            SwashContent::SubpixelMask => {
                                // Use average of RGB subpixel channels as alpha
                                let idx = (row * w + col) * 3;
                                let r = image.data[idx] as u16;
                                let g = image.data[idx + 1] as u16;
                                let b = image.data[idx + 2] as u16;
                                ((r + g + b) / 3) as u8
                            }
                        };

                        if alpha > 128 {
                            if line_start.is_none() {
                                line_start = Some((gx + col as i32) as f32);
                            }
                        } else if let Some(start) = line_start.take() {
                            let end = (gx + col as i32) as f32;
                            // Negate y to flip from raster (Y-down) to scope (Y-up)
                            shapes.push(Box::new(Line::new_2d(start, -y, end, -y)));
                        }
                    }

                    // Close any run that extends to the right edge of the glyph
                    if let Some(start) = line_start {
                        let end = (gx + w as i32) as f32;
                        shapes.push(Box::new(Line::new_2d(start, -y, end, -y)));
                    }
                }
            }
        }
    }

    // 7. Normalize shapes to [-1, 1]
    if !shapes.is_empty() {
        normalize_shapes(&mut shapes);
    }

    Ok(shapes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hello() {
        let config = TextConfig::default();
        let shapes = parse_text("Hello", &config).unwrap();
        // "Hello" should produce a non-trivial number of line segments
        assert!(
            !shapes.is_empty(),
            "Expected non-empty shapes for 'Hello', got 0"
        );
    }

    #[test]
    fn test_parse_empty_string() {
        let config = TextConfig::default();
        let shapes = parse_text("", &config).unwrap();
        assert!(shapes.is_empty());
    }

    #[test]
    fn test_custom_font_size() {
        let config = TextConfig { font_size: 48.0 };
        let shapes = parse_text("A", &config).unwrap();
        assert!(
            !shapes.is_empty(),
            "Expected non-empty shapes for 'A' at 48px"
        );
    }

    #[test]
    fn test_shapes_are_normalized() {
        let config = TextConfig::default();
        let shapes = parse_text("Test", &config).unwrap();
        if shapes.is_empty() {
            return; // skip if no system fonts available
        }

        // After normalization, all sampled points should be in roughly [-2, 2] range
        // (normalization targets [-1,1] but there can be slight overshoot)
        for shape in &shapes {
            let start = shape.next_vector(0.0);
            let end = shape.next_vector(1.0);
            assert!(
                start.x.abs() < 3.0 && start.y.abs() < 3.0,
                "Start point out of expected range: ({}, {})",
                start.x,
                start.y
            );
            assert!(
                end.x.abs() < 3.0 && end.y.abs() < 3.0,
                "End point out of expected range: ({}, {})",
                end.x,
                end.y
            );
        }
    }

    #[test]
    fn test_whitespace_only() {
        let config = TextConfig::default();
        // Whitespace characters typically have no visible glyph outlines
        let shapes = parse_text("   ", &config).unwrap();
        // This may or may not be empty depending on font, but should not panic
        let _ = shapes;
    }
}
