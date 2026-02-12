use crate::point::Point;
use crate::shape::{Shape, Line, total_length};

/// A frame is a collection of shapes that represents one "image" to be drawn.
///
/// In the synthesizer, each voice draws through a frame at a frequency,
/// converting the geometric shapes into audio samples.
pub struct Frame {
    pub shapes: Vec<Box<dyn Shape>>,
    pub total_length: f32,
}

impl Frame {
    pub fn new(shapes: Vec<Box<dyn Shape>>) -> Self {
        let total_length = total_length(&shapes);
        Self { shapes, total_length }
    }

    pub fn empty() -> Self {
        Self { shapes: Vec::new(), total_length: 0.0 }
    }

    /// Recompute the cached total length.
    pub fn recompute_length(&mut self) {
        self.total_length = total_length(&self.shapes);
    }

    /// Normalize all shapes to fit within [-1, 1].
    pub fn normalize(&mut self) {
        crate::shape::normalize_shapes(&mut self.shapes);
        self.recompute_length();
    }

    /// Normalize all shapes to fit within given dimensions.
    pub fn normalize_to(&mut self, width: f32, height: f32) {
        crate::shape::normalize_shapes_to(&mut self.shapes, width, height);
        self.remove_out_of_bounds();
        self.recompute_length();
    }

    /// Remove shapes whose endpoints are entirely out of bounds [-1, 1].
    pub fn remove_out_of_bounds(&mut self) {
        self.shapes.retain(|shape| {
            let start = shape.next_vector(0.0);
            let end = shape.next_vector(1.0);

            let start_in = (start.x > -1.0 && start.x < 1.0) || (start.y > -1.0 && start.y < 1.0);
            let end_in = (end.x > -1.0 && end.x < 1.0) || (end.y > -1.0 && end.y < 1.0);

            start_in && end_in
        });

        // Clip lines to bounds
        for shape in self.shapes.iter_mut() {
            if shape.shape_type() == "Line" {
                let start = shape.next_vector(0.0);
                let end = shape.next_vector(1.0);
                let new_start = Point::xy(
                    start.x.clamp(-1.0, 1.0),
                    start.y.clamp(-1.0, 1.0),
                );
                let new_end = Point::xy(
                    end.x.clamp(-1.0, 1.0),
                    end.y.clamp(-1.0, 1.0),
                );
                *shape = Box::new(Line::from_points(new_start, new_end));
            }
        }
    }

    /// Clone all shapes in this frame.
    pub fn clone_shapes(&self) -> Vec<Box<dyn Shape>> {
        self.shapes.iter().map(|s| s.clone_shape()).collect()
    }
}

impl Clone for Frame {
    fn clone(&self) -> Self {
        Self {
            shapes: self.clone_shapes(),
            total_length: self.total_length,
        }
    }
}
