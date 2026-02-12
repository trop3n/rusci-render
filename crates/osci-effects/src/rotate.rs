use osci_core::{EffectApplication, Point};

/// Rotate effect — rotates the input point around all three axes by amounts
/// proportional to the parameter values (scaled by PI).
#[derive(Debug, Clone)]
pub struct Rotate;

impl Rotate {
    pub fn new() -> Self {
        Self
    }
}

impl EffectApplication for Rotate {
    fn apply(
        &mut self,
        _index: usize,
        mut input: Point,
        _external_input: Point,
        values: &[f32],
        _sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        input.rotate(
            values[0] * std::f32::consts::PI,
            values[1] * std::f32::consts::PI,
            values[2] * std::f32::consts::PI,
        );
        input
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Rotate"
    }
}
