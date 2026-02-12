use osci_core::{EffectApplication, Point};

/// Ripple effect — adds a sinusoidal displacement to the Z axis based on
/// the distance from the origin in the XY plane, creating a ripple/wave pattern.
#[derive(Debug, Clone)]
pub struct Ripple;

impl Ripple {
    pub fn new() -> Self {
        Self
    }
}

impl EffectApplication for Ripple {
    fn apply(
        &mut self,
        _index: usize,
        mut input: Point,
        _external_input: Point,
        values: &[f32],
        _sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        let phase = values[1] * std::f32::consts::PI;
        let distance = 100.0 * values[2] * (input.x * input.x + input.y * input.y);

        input.z += values[0] * (phase + distance).sin();
        input
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Ripple"
    }
}
