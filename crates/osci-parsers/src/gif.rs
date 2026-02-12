use osci_core::shape::{Shape, normalize_shapes_to};

use super::image::ImageConfig;

/// A collection of parsed GIF frames, each containing oscilloscope shapes.
pub struct GifFrames {
    /// One entry per frame, each containing the shapes for that frame.
    pub frames: Vec<Vec<Box<dyn Shape>>>,
    /// Playback rate in frames per second, derived from frame delays.
    pub frame_rate: f64,
}

/// Parse an animated GIF from raw bytes into per-frame oscilloscope shapes.
///
/// Each frame is composited onto a full-size canvas, converted to grayscale,
/// and then threshold-scanned to produce horizontal line segments (using the
/// same algorithm as the image parser). Frame delays are averaged to compute
/// a playback frame rate.
pub fn parse_gif(data: &[u8], config: &ImageConfig) -> Result<GifFrames, String> {
    let mut decoder = gif::DecodeOptions::new();
    decoder.set_color_output(gif::ColorOutput::RGBA);
    let mut decoder = decoder
        .read_info(std::io::Cursor::new(data))
        .map_err(|e| format!("failed to decode GIF: {e}"))?;

    let global_width = decoder.width() as u32;
    let global_height = decoder.height() as u32;

    if global_width == 0 || global_height == 0 {
        return Ok(GifFrames {
            frames: Vec::new(),
            frame_rate: 10.0,
        });
    }

    let mut frames: Vec<Vec<Box<dyn Shape>>> = Vec::new();
    let mut total_delay: u64 = 0;
    let mut frame_count: u64 = 0;

    // Persistent canvas for frame compositing (RGBA)
    let canvas_size = (global_width * global_height * 4) as usize;
    let mut canvas = vec![0u8; canvas_size];

    while let Some(frame) = decoder.read_next_frame().map_err(|e| format!("GIF frame error: {e}"))? {
        let frame_left = frame.left as u32;
        let frame_top = frame.top as u32;
        let frame_width = frame.width as u32;
        let frame_height = frame.height as u32;

        // Composite frame onto the canvas at the correct offset
        for row in 0..frame_height {
            for col in 0..frame_width {
                let src_idx = ((row * frame_width + col) * 4) as usize;
                let dst_x = frame_left + col;
                let dst_y = frame_top + row;

                if dst_x < global_width && dst_y < global_height {
                    let dst_idx = ((dst_y * global_width + dst_x) * 4) as usize;
                    if src_idx + 3 < frame.buffer.len() && dst_idx + 3 < canvas.len() {
                        canvas[dst_idx] = frame.buffer[src_idx];
                        canvas[dst_idx + 1] = frame.buffer[src_idx + 1];
                        canvas[dst_idx + 2] = frame.buffer[src_idx + 2];
                        canvas[dst_idx + 3] = frame.buffer[src_idx + 3];
                    }
                }
            }
        }

        // Convert canvas to grayscale
        let num_pixels = (global_width * global_height) as usize;
        let mut gray_pixels = vec![0u8; num_pixels];
        for i in 0..num_pixels {
            let base = i * 4;
            if base + 2 < canvas.len() {
                let r = canvas[base] as f32;
                let g = canvas[base + 1] as f32;
                let b = canvas[base + 2] as f32;
                gray_pixels[i] = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
            }
        }

        let gray_image = image::GrayImage::from_raw(global_width, global_height, gray_pixels)
            .ok_or_else(|| "failed to create grayscale image from GIF frame".to_string())?;

        // Threshold scan to produce shapes
        let mut shapes = crate::image::threshold_scan(
            &gray_image,
            global_width,
            global_height,
            config,
        );

        normalize_shapes_to(&mut shapes, global_width as f32, global_height as f32);
        frames.push(shapes);

        // Accumulate delay (delay is in 1/100ths of a second)
        total_delay += frame.delay as u64;
        frame_count += 1;
    }

    // Calculate average frame rate from delays
    let frame_rate = if frame_count > 0 && total_delay > 0 {
        // total_delay is in 1/100 s units, average delay per frame
        let avg_delay_secs = (total_delay as f64) / (frame_count as f64) / 100.0;
        1.0 / avg_delay_secs
    } else {
        10.0 // default GIF frame rate
    };

    Ok(GifFrames { frames, frame_rate })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_gif_returns_error() {
        let result = parse_gif(b"not a gif", &ImageConfig::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_data_returns_error() {
        let result = parse_gif(&[], &ImageConfig::default());
        assert!(result.is_err());
    }

    #[test]
    fn test_gif_frames_struct() {
        let gif_frames = GifFrames {
            frames: vec![Vec::new(), Vec::new()],
            frame_rate: 10.0,
        };
        assert_eq!(gif_frames.frames.len(), 2);
        assert!((gif_frames.frame_rate - 10.0).abs() < 0.001);
    }
}
