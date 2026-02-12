use osci_core::shape::Shape;
use osci_core::Point;

/// Shape vector renderer — walks through a list of shapes, sampling points
/// along each shape at a rate determined by the drawing frequency.
///
/// Ported from C++ `ShapeVectorRenderer`. Given a frequency and sample rate,
/// the renderer computes how far along the total frame to advance each sample,
/// then interpolates the appropriate shape at the appropriate progress.
pub struct ShapeRenderer {
    sample_rate: f64,
    frequency: f64,

    shapes: Vec<Box<dyn Shape>>,
    shapes_length: f64,
    current_shape: usize,
    shape_drawn: f64,
    frame_drawn: f64,
}

impl ShapeRenderer {
    pub fn new(sample_rate: f64, frequency: f64) -> Self {
        Self {
            sample_rate,
            frequency,
            shapes: Vec::new(),
            shapes_length: 0.0,
            current_shape: 0,
            shape_drawn: 0.0,
            frame_drawn: 0.0,
        }
    }

    /// Replace the current shapes with new ones and reset drawing state.
    pub fn set_shapes(&mut self, shapes: Vec<Box<dyn Shape>>) {
        self.shapes_length = osci_core::shape::total_length(&shapes) as f64;
        self.shapes = shapes;
        self.current_shape = 0;
        self.shape_drawn = 0.0;
        self.frame_drawn = 0.0;
    }

    /// Set the sample rate.
    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
    }

    /// Set the drawing frequency.
    pub fn set_frequency(&mut self, frequency: f64) {
        self.frequency = frequency;
    }

    /// Get the total frame length.
    pub fn frame_length(&self) -> f64 {
        self.shapes_length
    }

    /// Returns true if there are no shapes to render.
    pub fn is_empty(&self) -> bool {
        self.shapes.is_empty()
    }

    /// Generate the next point in the shape sequence.
    ///
    /// Advances the drawing position by `length_increment` (derived from
    /// frequency and sample rate) and returns the interpolated point.
    pub fn next_vector(&mut self) -> Point {
        if self.shapes.is_empty() {
            return Point::new(0.0, 0.0, 1.0);
        }

        let point = if self.current_shape < self.shapes.len() {
            let shape = &self.shapes[self.current_shape];
            let length = shape.length() as f64;
            let progress = if length == 0.0 { 1.0 } else { self.shape_drawn / length };
            let mut p = shape.next_vector(progress as f32);
            p.z = 1.0;
            p
        } else {
            Point::new(0.0, 0.0, 1.0)
        };

        self.increment_shape_drawing();

        if self.frame_drawn >= self.shapes_length {
            self.frame_drawn -= self.shapes_length;
            self.current_shape = 0;
        }

        point
    }

    /// Generate the next point using an externally-provided length increment.
    ///
    /// This is used by the voice system which computes length_increment based
    /// on a potentially varying frequency.
    pub fn next_vector_with_increment(&mut self, length_increment: f64) -> Point {
        if self.shapes.is_empty() {
            return Point::new(0.0, 0.0, 1.0);
        }

        let point = if self.current_shape < self.shapes.len() {
            let shape = &self.shapes[self.current_shape];
            let length = shape.length() as f64;
            let progress = if length == 0.0 { 1.0 } else { self.shape_drawn / length };
            shape.next_vector(progress as f32)
        } else {
            Point::new(0.0, 0.0, 1.0)
        };

        self.increment_with(length_increment);

        point
    }

    /// Check if the frame has wrapped around, and if so, return true.
    /// The caller should update the frame when this happens.
    pub fn frame_complete(&self) -> bool {
        self.frame_drawn >= self.shapes_length && self.shapes_length > 0.0
    }

    /// Reset the frame-drawn counter after updating shapes.
    pub fn reset_frame_drawn(&mut self) {
        if self.shapes_length > 0.0 {
            self.frame_drawn -= self.shapes_length;
        }
        self.current_shape = 0;
    }

    fn increment_shape_drawing(&mut self) {
        if self.shapes.is_empty() {
            return;
        }

        let length_increment = if self.sample_rate > 0.0 {
            self.shapes_length / (self.sample_rate / self.frequency)
        } else {
            0.0
        };

        self.increment_with(length_increment);
    }

    fn increment_with(&mut self, length_increment: f64) {
        if self.shapes.is_empty() {
            return;
        }

        let mut length = if self.current_shape < self.shapes.len() {
            self.shapes[self.current_shape].length() as f64
        } else {
            0.0
        };

        self.frame_drawn += length_increment;
        self.shape_drawn += length_increment;

        // Skip over shapes that the increment draws past
        while self.shape_drawn > length && !self.shapes.is_empty() {
            self.shape_drawn -= length;
            self.current_shape += 1;
            if self.current_shape >= self.shapes.len() {
                self.current_shape = 0;
            }
            length = self.shapes[self.current_shape].length() as f64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use osci_core::shape::Line;

    #[test]
    fn test_renderer_empty() {
        let mut r = ShapeRenderer::new(44100.0, 440.0);
        let p = r.next_vector();
        assert!((p.x).abs() < 0.001);
        assert!((p.z - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_renderer_with_line() {
        let mut r = ShapeRenderer::new(44100.0, 60.0);
        let line = Line::from_points(
            Point::new(-1.0, -1.0, 0.0),
            Point::new(1.0, 1.0, 0.0),
        );
        r.set_shapes(vec![Box::new(line)]);

        // Should produce points along the line
        let p1 = r.next_vector();
        assert!(p1.x >= -1.0 && p1.x <= 1.0);
    }

    #[test]
    fn test_frame_length() {
        let mut r = ShapeRenderer::new(44100.0, 60.0);
        let line = Line::from_points(
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
        );
        r.set_shapes(vec![Box::new(line)]);
        assert!(r.frame_length() > 0.0);
    }
}
