use osci_core::{EffectApplication, Point};

/// Skew effect — sequential shear transformations.
///
/// Applies shearing along each axis based on the perpendicular axis
/// coordinates, producing a skew distortion.
#[derive(Debug, Clone)]
pub struct SkewEffect;

impl SkewEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectApplication for SkewEffect {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        _sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        let mut out = input;

        out.x += values[0] * input.y;
        out.y += values[1] * input.z;
        out.z += values[2] * input.x;

        out
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Skew"
    }
}
