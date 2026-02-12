use osci_core::{EffectApplication, Point};
use std::f64::consts::{PI, TAU};

/// Polygonizer effect — polar quantization to polygon/stripe grid.
///
/// Quantizes the input in polar coordinates to form regular polygon
/// shapes, combined with radial stripe quantization.
#[derive(Debug, Clone)]
pub struct PolygonizerEffect;

impl PolygonizerEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectApplication for PolygonizerEffect {
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
        let n_sides = values[1].max(2.0) as f64;
        let stripe_size_param = values[2].max(1e-4) as f64;
        let stripe_size = (0.63 * stripe_size_param).powf(1.5);
        let rotation = values[3] as f64 * TAU;
        let stripe_phase = values[4] as f64;

        let x = input.x as f64;
        let y = input.y as f64;
        let z = input.z as f64;

        let r = x.hypot(y);
        let mut theta = (-x).atan2(y) - rotation;

        // Wrap theta to [-PI, PI]
        theta = ((theta + PI) % TAU + TAU) % TAU - PI;

        // Quantize angle to polygon
        let region_center_theta = (theta * n_sides / TAU).round() / n_sides * TAU;

        // Distance from center along the polygon edge direction
        let dist = r * (theta - region_center_theta).cos();

        // Quantize distance to stripes
        let new_dist = ((dist / stripe_size - stripe_phase).round() + stripe_phase) * stripe_size;
        let new_dist = new_dist.max(0.0);

        let scale = if dist.abs() > 1e-10 {
            new_dist / dist
        } else {
            1.0
        };

        // Quantize z in the same stripe pattern
        let abs_z = z.abs();
        let new_z = if abs_z > 1e-10 {
            let quantized_z = ((abs_z / stripe_size - stripe_phase).round() + stripe_phase) * stripe_size;
            quantized_z.max(0.0) * z.signum()
        } else {
            z
        };

        let out_x = (x * scale) as f32;
        let out_y = (y * scale) as f32;
        let out_z = new_z as f32;

        Point::with_rgb(
            (1.0 - effect_scale) * input.x + effect_scale * out_x,
            (1.0 - effect_scale) * input.y + effect_scale * out_y,
            (1.0 - effect_scale) * input.z + effect_scale * out_z,
            input.r,
            input.g,
            input.b,
        )
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Polygonizer"
    }
}
