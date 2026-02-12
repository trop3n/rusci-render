use osci_core::{EffectApplication, Point};
use std::f32::consts::PI;

/// Twist effect — Y-dependent rotation.
///
/// Rotates points around the Y axis by an angle proportional to their
/// Y coordinate, producing a twisting distortion along the vertical axis.
#[derive(Debug, Clone)]
pub struct TwistEffect;

impl TwistEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectApplication for TwistEffect {
    fn apply(
        &mut self,
        _index: usize,
        mut input: Point,
        _external_input: Point,
        values: &[f32],
        _sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        let twist_strength = values[0] * 4.0 * PI;
        let twist_theta = twist_strength * input.y;

        input.rotate(0.0, twist_theta, 0.0);
        input
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Twist"
    }
}
