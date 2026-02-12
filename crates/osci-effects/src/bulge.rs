use osci_core::{EffectApplication, Point};

/// Bulge effect — applies a radial power-law distortion in the XY plane.
///
/// Points closer to or farther from the origin are pushed/pulled based on the
/// `translatedBulge` exponent derived from the parameter value.
#[derive(Debug, Clone)]
pub struct Bulge;

impl Bulge {
    pub fn new() -> Self {
        Self
    }
}

impl EffectApplication for Bulge {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        _sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        let value = values[0];
        let translated_bulge = -value + 1.0;

        let r = input.x.hypot(input.y);
        if r == 0.0 {
            return input;
        }

        let rn = r.powf(translated_bulge);
        let scale = rn / r;

        Point::with_rgb(
            scale * input.x,
            scale * input.y,
            input.z,
            input.r,
            input.g,
            input.b,
        )
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Bulge"
    }
}
