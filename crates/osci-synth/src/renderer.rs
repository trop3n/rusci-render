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

        if self.frame_complete() {
            self.wrap_frame();
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

    /// Swap in the next frame's shapes once `frame_complete()` is true.
    ///
    /// Any distance drawn past the end of the old frame carries over into
    /// the new one, so the beam keeps moving at a steady speed.
    pub fn start_next_frame(&mut self, shapes: Vec<Box<dyn Shape>>) {
        let overshoot = (self.frame_drawn - self.shapes_length).max(0.0);
        self.set_shapes(shapes);
        self.move_to(overshoot);
    }

    /// Go back to the start of the same frame, keeping the overshoot.
    fn wrap_frame(&mut self) {
        let overshoot = (self.frame_drawn - self.shapes_length).max(0.0);
        self.move_to(overshoot);
    }

    /// Place the beam `distance` along the frame from its start.
    fn move_to(&mut self, distance: f64) {
        self.current_shape = 0;
        self.frame_drawn = 0.0;
        self.shape_drawn = 0.0;
        if self.shapes_length > 0.0 {
            // Wrap first so a huge overshoot can't loop over the shapes many times.
            self.increment_with(distance % self.shapes_length);
        }
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

    fn square() -> Vec<Box<dyn Shape>> {
        let corners = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
        (0..4)
            .map(|i| {
                let (x1, y1) = corners[i];
                let (x2, y2) = corners[(i + 1) % 4];
                Box::new(Line::new_2d(x1, y1, x2, y2)) as Box<dyn Shape>
            })
            .collect()
    }

    fn close(a: Point, b: Point) -> bool {
        (a.x - b.x).abs() < 1e-3 && (a.y - b.y).abs() < 1e-3
    }

    #[test]
    fn wrap_returns_to_start_of_first_shape() {
        // 64 samples per trip round the square.
        let mut r = ShapeRenderer::new(6400.0, 100.0);
        r.set_shapes(square());
        let first = r.next_vector();
        for _ in 1..64 {
            r.next_vector();
        }
        let after_wrap = r.next_vector();
        assert!(close(first, after_wrap), "{first:?} vs {after_wrap:?}");
    }

    #[test]
    fn output_is_periodic_over_many_frames() {
        let mut r = ShapeRenderer::new(6400.0, 100.0);
        r.set_shapes(square());
        let samples: Vec<Point> = (0..64 * 10).map(|_| r.next_vector()).collect();
        for (i, p) in samples.iter().enumerate().skip(64) {
            assert!(close(*p, samples[i - 64]), "sample {i}: {p:?} vs {:?}", samples[i - 64]);
        }
    }

    #[test]
    fn next_frame_completes_after_one_trip() {
        // The voice path: drive with an increment, swap frames on completion.
        let mut r = ShapeRenderer::new(6400.0, 100.0);
        r.set_shapes(square());
        let increment = r.frame_length() / 64.0;
        let mut completions = Vec::new();
        for i in 0..64 * 4 {
            r.next_vector_with_increment(increment);
            if r.frame_complete() {
                completions.push(i);
                r.start_next_frame(square());
            }
        }
        assert_eq!(completions, vec![63, 127, 191, 255]);
    }

    #[test]
    fn next_frame_keeps_the_overshoot() {
        let mut r = ShapeRenderer::new(6400.0, 100.0);
        r.set_shapes(square());
        // Square length is 8; 3 steps of 3 overshoot the end by 1.
        for _ in 0..3 {
            r.next_vector_with_increment(3.0);
        }
        assert!(r.frame_complete());
        r.start_next_frame(square());
        // 1 unit along the first edge, from (-1,-1) towards (1,-1).
        let p = r.next_vector_with_increment(0.0);
        assert!(close(p, Point::xy(0.0, -1.0)), "{p:?}");
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
