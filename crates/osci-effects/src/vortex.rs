use osci_core::{EffectApplication, Point};
use std::f64::consts::TAU;

/// Vortex effect — complex exponentiation z^n.
///
/// Treats the XY plane as the complex plane and raises each point to
/// the nth power, producing vortex-like distortion patterns.
#[derive(Debug, Clone)]
pub struct VortexEffect;

impl VortexEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectApplication for VortexEffect {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        _sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        let effect_scale = values[0].clamp(0.0, 1.0);
        let exponent = (values[1] + 0.001).floor().max(1.0) as f64;
        let ref_theta = values[2] as f64 * TAU;

        let x = input.x as f64;
        let y = input.y as f64;

        let r2 = x * x + y * y;
        let theta = y.atan2(x) - ref_theta;

        let out_r = r2.powf(0.5 * exponent);
        let out_theta = exponent * theta + ref_theta;

        let out_x = (out_r * out_theta.cos()) as f32;
        let out_y = (out_r * out_theta.sin()) as f32;

        Point::with_rgb(
            (1.0 - effect_scale) * input.x + effect_scale * out_x,
            (1.0 - effect_scale) * input.y + effect_scale * out_y,
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
        "Vortex"
    }
}
