use osci_core::{EffectApplication, Point};

/// Swirl effect — rotates points in the XY plane by an angle proportional to
/// their distance from the origin, producing a spiral/swirl distortion.
#[derive(Debug, Clone)]
pub struct Swirl;

impl Swirl {
    pub fn new() -> Self {
        Self
    }
}

impl EffectApplication for Swirl {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        _sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        let length = 10.0 * values[0] * input.magnitude();

        let cos_l = length.cos();
        let sin_l = length.sin();

        let new_x = input.x * cos_l - input.y * sin_l;
        let new_y = input.x * sin_l + input.y * cos_l;

        Point::with_rgb(new_x, new_y, input.z, input.r, input.g, input.b)
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Swirl"
    }
}
