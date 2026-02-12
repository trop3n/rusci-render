use crate::point::Point;

/// A drawable shape that can be sampled at any drawing progress [0, 1].
///
/// Mirrors the C++ `osci::Shape` interface. Shapes are the fundamental
/// geometric primitives: lines, bezier curves, arcs, and point shapes.
pub trait Shape: Send + Sync {
    /// Sample the shape at a given drawing progress in [0, 1].
    fn next_vector(&self, drawing_progress: f32) -> Point;

    /// Scale the shape's coordinates.
    fn scale(&mut self, x: f32, y: f32, z: f32);

    /// Translate the shape's coordinates.
    fn translate(&mut self, x: f32, y: f32, z: f32);

    /// The path length of this shape. Returns a cached value after first computation.
    fn length(&self) -> f32;

    /// Clone this shape into a boxed trait object.
    fn clone_shape(&self) -> Box<dyn Shape>;

    /// Shape type name for debugging.
    fn shape_type(&self) -> &'static str;
}

/// Compute total path length of a collection of shapes.
pub fn total_length(shapes: &[Box<dyn Shape>]) -> f32 {
    shapes.iter().map(|s| s.length()).sum()
}

/// Normalize shapes to fit within [-1, 1] coordinate range.
pub fn normalize_shapes(shapes: &mut [Box<dyn Shape>]) {
    let h = shapes_height(shapes);
    let w = shapes_width(shapes);
    let max_dim = h.max(w);

    if max_dim == 0.0 {
        return;
    }

    for shape in shapes.iter_mut() {
        shape.scale(2.0 / max_dim, -2.0 / max_dim, 2.0 / max_dim);
    }

    let max_pt = max_vector(shapes);
    let new_height = shapes_height(shapes);

    for shape in shapes.iter_mut() {
        shape.translate(-1.0, -max_pt.y + new_height / 2.0, 0.0);
    }
}

/// Normalize shapes to fit within a given width/height.
pub fn normalize_shapes_to(shapes: &mut [Box<dyn Shape>], width: f32, height: f32) {
    let max_dim = width.max(height);

    if max_dim == 0.0 {
        return;
    }

    for shape in shapes.iter_mut() {
        shape.scale(2.0 / max_dim, -2.0 / max_dim, 2.0 / max_dim);
        shape.translate(-1.0, 1.0, 0.0);
    }

    remove_out_of_bounds(shapes);
}

/// Compute the height (Y range) of a set of shapes by sampling.
pub fn shapes_height(shapes: &[Box<dyn Shape>]) -> f32 {
    let mut max_y = f32::MIN;
    let mut min_y = f32::MAX;

    for shape in shapes {
        for i in 0..4 {
            let v = shape.next_vector(i as f32 / 4.0);
            max_y = max_y.max(v.y);
            min_y = min_y.min(v.y);
        }
    }

    (max_y - min_y).abs()
}

/// Compute the width (X range) of a set of shapes by sampling.
pub fn shapes_width(shapes: &[Box<dyn Shape>]) -> f32 {
    let mut max_x = f32::MIN;
    let mut min_x = f32::MAX;

    for shape in shapes {
        for i in 0..4 {
            let v = shape.next_vector(i as f32 / 4.0);
            max_x = max_x.max(v.x);
            min_x = min_x.min(v.x);
        }
    }

    (max_x - min_x).abs()
}

/// Find the maximum X and Y values among shape endpoints.
pub fn max_vector(shapes: &[Box<dyn Shape>]) -> Point {
    let mut max_x = f32::MIN;
    let mut max_y = f32::MIN;

    for shape in shapes {
        let start = shape.next_vector(0.0);
        let end = shape.next_vector(1.0);

        max_x = max_x.max(start.x).max(end.x);
        max_y = max_y.max(start.y).max(end.y);
    }

    Point::xy(max_x, max_y)
}

/// Remove shapes whose endpoints are entirely out of the [-1, 1] bounds.
/// Line shapes that partially overlap are clipped.
fn remove_out_of_bounds(shapes: &mut [Box<dyn Shape>]) {
    // Note: we operate on a Vec in frame.rs; this is a helper for normalize_shapes_to.
    // Since we can't easily resize a slice, this is a no-op on slices.
    // The actual removal happens in Frame::remove_out_of_bounds.
    let _ = shapes;
}

// --- Concrete shape implementations ---

/// A line segment between two 3D points.
#[derive(Debug, Clone)]
pub struct Line {
    pub x1: f32,
    pub y1: f32,
    pub z1: f32,
    pub x2: f32,
    pub y2: f32,
    pub z2: f32,
    cached_length: Option<f32>,
}

impl Line {
    pub fn new_2d(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, z1: 0.0, x2, y2, z2: 0.0, cached_length: None }
    }

    pub fn new_3d(x1: f32, y1: f32, z1: f32, x2: f32, y2: f32, z2: f32) -> Self {
        Self { x1, y1, z1, x2, y2, z2, cached_length: None }
    }

    pub fn from_points(p1: Point, p2: Point) -> Self {
        Self { x1: p1.x, y1: p1.y, z1: p1.z, x2: p2.x, y2: p2.y, z2: p2.z, cached_length: None }
    }

    pub fn compute_length(x1: f32, y1: f32, z1: f32, x2: f32, y2: f32, z2: f32) -> f32 {
        let dx = x2 - x1;
        let dy = y2 - y1;
        let dz = z2 - z1;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

impl Shape for Line {
    fn next_vector(&self, drawing_progress: f32) -> Point {
        Point::new(
            self.x1 + (self.x2 - self.x1) * drawing_progress,
            self.y1 + (self.y2 - self.y1) * drawing_progress,
            self.z1 + (self.z2 - self.z1) * drawing_progress,
        )
    }

    fn scale(&mut self, x: f32, y: f32, z: f32) {
        self.x1 *= x; self.y1 *= y; self.z1 *= z;
        self.x2 *= x; self.y2 *= y; self.z2 *= z;
        self.cached_length = None;
    }

    fn translate(&mut self, x: f32, y: f32, z: f32) {
        self.x1 += x; self.y1 += y; self.z1 += z;
        self.x2 += x; self.y2 += y; self.z2 += z;
    }

    fn length(&self) -> f32 {
        // Use cached length if available. Since &self is immutable,
        // we compute on the fly if not cached.
        self.cached_length.unwrap_or_else(|| {
            Self::compute_length(self.x1, self.y1, self.z1, self.x2, self.y2, self.z2)
        })
    }

    fn clone_shape(&self) -> Box<dyn Shape> {
        Box::new(self.clone())
    }

    fn shape_type(&self) -> &'static str {
        "Line"
    }
}

/// A cubic Bezier curve defined by 4 control points (2D).
#[derive(Debug, Clone)]
pub struct CubicBezierCurve {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub x3: f32,
    pub y3: f32,
    pub x4: f32,
    pub y4: f32,
    cached_length: Option<f32>,
}

impl CubicBezierCurve {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32, x4: f32, y4: f32) -> Self {
        Self { x1, y1, x2, y2, x3, y3, x4, y4, cached_length: None }
    }
}

impl Shape for CubicBezierCurve {
    fn next_vector(&self, t: f32) -> Point {
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        let mt3 = mt2 * mt;
        let t2 = t * t;
        let t3 = t2 * t;

        let x = mt3 * self.x1 + 3.0 * mt2 * t * self.x2 + 3.0 * mt * t2 * self.x3 + t3 * self.x4;
        let y = mt3 * self.y1 + 3.0 * mt2 * t * self.y2 + 3.0 * mt * t2 * self.y3 + t3 * self.y4;

        Point::xy(x, y)
    }

    fn scale(&mut self, x: f32, y: f32, _z: f32) {
        self.x1 *= x; self.y1 *= y;
        self.x2 *= x; self.y2 *= y;
        self.x3 *= x; self.y3 *= y;
        self.x4 *= x; self.y4 *= y;
        self.cached_length = None;
    }

    fn translate(&mut self, x: f32, y: f32, _z: f32) {
        self.x1 += x; self.y1 += y;
        self.x2 += x; self.y2 += y;
        self.x3 += x; self.y3 += y;
        self.x4 += x; self.y4 += y;
    }

    fn length(&self) -> f32 {
        self.cached_length.unwrap_or_else(|| {
            // Octagonal boundary approximation (matches C++)
            let dx = (self.x4 - self.x1).abs();
            let dy = (self.y4 - self.y1).abs();
            0.41 * dx.min(dy) + 0.941246 * dx.max(dy)
        })
    }

    fn clone_shape(&self) -> Box<dyn Shape> {
        Box::new(self.clone())
    }

    fn shape_type(&self) -> &'static str {
        "CubicBezierCurve"
    }
}

/// A quadratic Bezier curve, stored as a cubic Bezier using degree elevation.
#[derive(Debug, Clone)]
pub struct QuadraticBezierCurve {
    inner: CubicBezierCurve,
}

impl QuadraticBezierCurve {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32, x3: f32, y3: f32) -> Self {
        // Degree elevation: quadratic → cubic
        let cx2 = x1 + (x2 - x1) * (2.0 / 3.0);
        let cy2 = y1 + (y2 - y1) * (2.0 / 3.0);
        let cx3 = x3 + (x2 - x3) * (2.0 / 3.0);
        let cy3 = y3 + (y2 - y3) * (2.0 / 3.0);
        Self {
            inner: CubicBezierCurve::new(x1, y1, cx2, cy2, cx3, cy3, x3, y3),
        }
    }
}

impl Shape for QuadraticBezierCurve {
    fn next_vector(&self, drawing_progress: f32) -> Point {
        self.inner.next_vector(drawing_progress)
    }

    fn scale(&mut self, x: f32, y: f32, z: f32) {
        self.inner.scale(x, y, z);
    }

    fn translate(&mut self, x: f32, y: f32, z: f32) {
        self.inner.translate(x, y, z);
    }

    fn length(&self) -> f32 {
        self.inner.length()
    }

    fn clone_shape(&self) -> Box<dyn Shape> {
        Box::new(self.clone())
    }

    fn shape_type(&self) -> &'static str {
        "QuadraticBezierCurve"
    }
}

/// A circular or elliptical arc.
#[derive(Debug, Clone)]
pub struct CircleArc {
    pub x: f32,
    pub y: f32,
    pub radius_x: f32,
    pub radius_y: f32,
    pub start_angle: f32,
    pub end_angle: f32,
    cached_length: Option<f32>,
}

impl CircleArc {
    pub fn new(x: f32, y: f32, radius_x: f32, radius_y: f32, start_angle: f32, end_angle: f32) -> Self {
        Self { x, y, radius_x, radius_y, start_angle, end_angle, cached_length: None }
    }
}

impl Shape for CircleArc {
    fn next_vector(&self, drawing_progress: f32) -> Point {
        let angle = self.start_angle + self.end_angle * drawing_progress;
        Point::xy(
            self.x + self.radius_x * angle.cos(),
            self.y + self.radius_y * angle.sin(),
        )
    }

    fn scale(&mut self, x: f32, y: f32, _z: f32) {
        self.x *= x;
        self.y *= y;
        self.radius_x *= x;
        self.radius_y *= y;
        self.cached_length = None;
    }

    fn translate(&mut self, x: f32, y: f32, _z: f32) {
        self.x += x;
        self.y += y;
    }

    fn length(&self) -> f32 {
        self.cached_length.unwrap_or_else(|| {
            // Approximate by sampling 5 line segments (matches C++)
            let segments = 5;
            let mut len = 0.0;
            let mut end = self.next_vector(0.0);
            for i in 0..segments {
                let start = end;
                end = self.next_vector((i + 1) as f32 / segments as f32);
                len += Line::compute_length(start.x, start.y, start.z, end.x, end.y, end.z);
            }
            len
        })
    }

    fn clone_shape(&self) -> Box<dyn Shape> {
        Box::new(self.clone())
    }

    fn shape_type(&self) -> &'static str {
        "Arc"
    }
}

/// A single-point "shape" that always returns the same point.
#[derive(Debug, Clone)]
pub struct PointShape {
    pub point: Point,
}

impl PointShape {
    pub fn new(point: Point) -> Self {
        Self { point }
    }
}

impl Shape for PointShape {
    fn next_vector(&self, _drawing_progress: f32) -> Point {
        self.point
    }

    fn scale(&mut self, x: f32, y: f32, z: f32) {
        self.point.scale(x, y, z);
    }

    fn translate(&mut self, x: f32, y: f32, z: f32) {
        self.point.translate(x, y, z);
    }

    fn length(&self) -> f32 {
        0.0
    }

    fn clone_shape(&self) -> Box<dyn Shape> {
        Box::new(self.clone())
    }

    fn shape_type(&self) -> &'static str {
        "Point"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_interpolation() {
        let line = Line::new_2d(0.0, 0.0, 10.0, 10.0);
        let mid = line.next_vector(0.5);
        assert!((mid.x - 5.0).abs() < 0.001);
        assert!((mid.y - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_line_length() {
        let line = Line::new_2d(0.0, 0.0, 3.0, 4.0);
        assert!((line.length() - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_cubic_bezier_endpoints() {
        let curve = CubicBezierCurve::new(0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0);
        let start = curve.next_vector(0.0);
        let end = curve.next_vector(1.0);
        assert!((start.x).abs() < 0.001);
        assert!((start.y).abs() < 0.001);
        assert!((end.x).abs() < 0.001);
        assert!((end.y - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_circle_arc() {
        let arc = CircleArc::new(0.0, 0.0, 1.0, 1.0, 0.0, std::f32::consts::TAU);
        let start = arc.next_vector(0.0);
        assert!((start.x - 1.0).abs() < 0.001);
        assert!((start.y).abs() < 0.001);
    }

    #[test]
    fn test_total_length() {
        let shapes: Vec<Box<dyn Shape>> = vec![
            Box::new(Line::new_2d(0.0, 0.0, 3.0, 4.0)),
            Box::new(Line::new_2d(0.0, 0.0, 6.0, 8.0)),
        ];
        let total = total_length(&shapes);
        assert!((total - 15.0).abs() < 0.001);
    }
}
