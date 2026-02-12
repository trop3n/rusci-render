use std::ops::{Add, AddAssign, Div, DivAssign, Index, IndexMut, Mul, MulAssign, Neg, Sub, SubAssign};

/// A point in 3D space with optional RGB color channels.
///
/// Matches the C++ `osci::Point` which has x, y, z spatial coordinates
/// plus r, g, b color channels (0..1). The z channel also serves as
/// legacy brightness/intensity when RGB is not explicitly specified.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

const EPSILON: f32 = 0.0001;

impl Default for Point {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Point {
    pub const ZERO: Point = Point {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        // Legacy: z replicated to RGB
        Self { x, y, z, r: z, g: z, b: z }
    }

    pub fn xy(x: f32, y: f32) -> Self {
        Self { x, y, z: 0.0, r: 0.0, g: 0.0, b: 0.0 }
    }

    pub fn with_rgb(x: f32, y: f32, z: f32, r: f32, g: f32, b: f32) -> Self {
        Self { x, y, z, r, g, b }
    }

    pub fn splat(val: f32) -> Self {
        Self { x: val, y: val, z: 0.0, r: 0.0, g: 0.0, b: 0.0 }
    }

    /// Attach color to an existing point (non-mutating)
    pub fn with_colour(&self, r: f32, g: f32, b: f32) -> Self {
        Self { x: self.x, y: self.y, z: self.z, r, g, b }
    }

    pub fn magnitude(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalize(&mut self) {
        let mag = self.magnitude();
        if mag > 0.0 {
            self.x /= mag;
            self.y /= mag;
            self.z /= mag;
        }
    }

    pub fn inner_product(&self, other: &Point) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn rotate(&mut self, rotate_x: f32, rotate_y: f32, rotate_z: f32) {
        // Rotate around x-axis
        let cos_val = rotate_x.cos();
        let sin_val = rotate_x.sin();
        let y2 = cos_val * self.y - sin_val * self.z;
        let z2 = sin_val * self.y + cos_val * self.z;

        // Rotate around y-axis
        let cos_val = rotate_y.cos();
        let sin_val = rotate_y.sin();
        let x2 = cos_val * self.x + sin_val * z2;
        self.z = -sin_val * self.x + cos_val * z2;

        // Rotate around z-axis
        let cos_val = rotate_z.cos();
        let sin_val = rotate_z.sin();
        self.x = cos_val * x2 - sin_val * y2;
        self.y = sin_val * x2 + cos_val * y2;
    }

    pub fn scale(&mut self, x: f32, y: f32, z: f32) {
        self.x *= x;
        self.y *= y;
        self.z *= z;
    }

    pub fn translate(&mut self, x: f32, y: f32, z: f32) {
        self.x += x;
        self.y += y;
        self.z += z;
    }

    /// Approximate equality using epsilon comparison
    pub fn approx_eq(&self, other: &Point) -> bool {
        (self.x - other.x).abs() < EPSILON
            && (self.y - other.y).abs() < EPSILON
            && (self.z - other.z).abs() < EPSILON
            && (self.r - other.r).abs() < EPSILON
            && (self.g - other.g).abs() < EPSILON
            && (self.b - other.b).abs() < EPSILON
    }
}

// Point + Point
impl Add for Point {
    type Output = Point;
    fn add(self, rhs: Point) -> Point {
        Point::with_rgb(
            self.x + rhs.x, self.y + rhs.y, self.z + rhs.z,
            self.r + rhs.r, self.g + rhs.g, self.b + rhs.b,
        )
    }
}

// Point + f32 (scalar adds to xyz only)
impl Add<f32> for Point {
    type Output = Point;
    fn add(self, rhs: f32) -> Point {
        Point::with_rgb(self.x + rhs, self.y + rhs, self.z + rhs, self.r, self.g, self.b)
    }
}

// f32 + Point
impl Add<Point> for f32 {
    type Output = Point;
    fn add(self, rhs: Point) -> Point {
        Point::with_rgb(rhs.x + self, rhs.y + self, rhs.z + self, rhs.r, rhs.g, rhs.b)
    }
}

// Point - Point
impl Sub for Point {
    type Output = Point;
    fn sub(self, rhs: Point) -> Point {
        Point::with_rgb(
            self.x - rhs.x, self.y - rhs.y, self.z - rhs.z,
            self.r - rhs.r, self.g - rhs.g, self.b - rhs.b,
        )
    }
}

// Point - f32
impl Sub<f32> for Point {
    type Output = Point;
    fn sub(self, rhs: f32) -> Point {
        Point::with_rgb(self.x - rhs, self.y - rhs, self.z - rhs, self.r, self.g, self.b)
    }
}

// -Point
impl Neg for Point {
    type Output = Point;
    fn neg(self) -> Point {
        Point::with_rgb(-self.x, -self.y, -self.z, self.r, self.g, self.b)
    }
}

// Point * Point (element-wise)
impl Mul for Point {
    type Output = Point;
    fn mul(self, rhs: Point) -> Point {
        Point::with_rgb(
            self.x * rhs.x, self.y * rhs.y, self.z * rhs.z,
            self.r * rhs.r, self.g * rhs.g, self.b * rhs.b,
        )
    }
}

// Point * f32 (scalar multiplies xyz only, per C++ behavior for `operator*`)
impl Mul<f32> for Point {
    type Output = Point;
    fn mul(self, rhs: f32) -> Point {
        Point::with_rgb(self.x * rhs, self.y * rhs, self.z * rhs, self.r, self.g, self.b)
    }
}

// f32 * Point
impl Mul<Point> for f32 {
    type Output = Point;
    fn mul(self, rhs: Point) -> Point {
        Point::with_rgb(rhs.x * self, rhs.y * self, rhs.z * self, rhs.r, rhs.g, rhs.b)
    }
}

// Point / f32
impl Div<f32> for Point {
    type Output = Point;
    fn div(self, rhs: f32) -> Point {
        Point::with_rgb(self.x / rhs, self.y / rhs, self.z / rhs, self.r, self.g, self.b)
    }
}

// Compound assignment operators

impl AddAssign for Point {
    fn add_assign(&mut self, rhs: Point) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
        self.r += rhs.r;
        self.g += rhs.g;
        self.b += rhs.b;
    }
}

impl AddAssign<f32> for Point {
    fn add_assign(&mut self, rhs: f32) {
        self.x += rhs;
        self.y += rhs;
        self.z += rhs;
    }
}

impl SubAssign for Point {
    fn sub_assign(&mut self, rhs: Point) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
        self.r -= rhs.r;
        self.g -= rhs.g;
        self.b -= rhs.b;
    }
}

impl SubAssign<f32> for Point {
    fn sub_assign(&mut self, rhs: f32) {
        self.x -= rhs;
        self.y -= rhs;
        self.z -= rhs;
    }
}

impl MulAssign for Point {
    fn mul_assign(&mut self, rhs: Point) {
        self.x *= rhs.x;
        self.y *= rhs.y;
        self.z *= rhs.z;
        self.r *= rhs.r;
        self.g *= rhs.g;
        self.b *= rhs.b;
    }
}

// *= f32 multiplies ALL channels (matching C++ `operator*=` scalar behavior)
impl MulAssign<f32> for Point {
    fn mul_assign(&mut self, rhs: f32) {
        self.x *= rhs;
        self.y *= rhs;
        self.z *= rhs;
        self.r *= rhs;
        self.g *= rhs;
        self.b *= rhs;
    }
}

impl DivAssign<f32> for Point {
    fn div_assign(&mut self, rhs: f32) {
        self.x /= rhs;
        self.y /= rhs;
        self.z /= rhs;
        self.r /= rhs;
        self.g /= rhs;
        self.b /= rhs;
    }
}

// Index access: 0=x, 1=y, 2=z, 3=r, 4=g, 5=b
impl Index<usize> for Point {
    type Output = f32;
    fn index(&self, index: usize) -> &f32 {
        match index {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            3 => &self.r,
            4 => &self.g,
            5 => &self.b,
            _ => panic!("Point index out of bounds: {index}, must be 0-5"),
        }
    }
}

impl IndexMut<usize> for Point {
    fn index_mut(&mut self, index: usize) -> &mut f32 {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            3 => &mut self.r,
            4 => &mut self.g,
            5 => &mut self.b,
            _ => panic!("Point index out of bounds: {index}, must be 0-5"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_default() {
        let p = Point::default();
        assert_eq!(p.x, 0.0);
        assert_eq!(p.y, 0.0);
        assert_eq!(p.z, 0.0);
    }

    #[test]
    fn test_point_new_z_to_rgb() {
        let p = Point::new(1.0, 2.0, 0.5);
        assert_eq!(p.r, 0.5);
        assert_eq!(p.g, 0.5);
        assert_eq!(p.b, 0.5);
    }

    #[test]
    fn test_point_add() {
        let a = Point::new(1.0, 2.0, 3.0);
        let b = Point::new(4.0, 5.0, 6.0);
        let c = a + b;
        assert_eq!(c.x, 5.0);
        assert_eq!(c.y, 7.0);
        assert_eq!(c.z, 9.0);
    }

    #[test]
    fn test_point_magnitude() {
        let p = Point::new(3.0, 4.0, 0.0);
        assert!((p.magnitude() - 5.0).abs() < EPSILON);
    }

    #[test]
    fn test_point_rotate() {
        let mut p = Point::xy(1.0, 0.0);
        p.rotate(0.0, 0.0, std::f32::consts::FRAC_PI_2);
        assert!((p.x).abs() < 0.001);
        assert!((p.y - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_point_negate() {
        let p = Point::new(1.0, -2.0, 3.0);
        let n = -p;
        assert_eq!(n.x, -1.0);
        assert_eq!(n.y, 2.0);
        assert_eq!(n.z, -3.0);
        // Color is preserved (not negated)
        assert_eq!(n.r, p.r);
    }

    #[test]
    fn test_point_index() {
        let p = Point::with_rgb(1.0, 2.0, 3.0, 0.1, 0.2, 0.3);
        assert_eq!(p[0], 1.0);
        assert_eq!(p[1], 2.0);
        assert_eq!(p[2], 3.0);
        assert_eq!(p[3], 0.1);
        assert_eq!(p[4], 0.2);
        assert_eq!(p[5], 0.3);
    }
}
