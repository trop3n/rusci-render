use osci_core::{EffectApplication, Point};
use std::f64::consts::TAU;

/// SpiralBitcrush effect — log-polar quantization.
///
/// Converts coordinates to log-polar space, quantizes them along a
/// spiral grid defined by domain parameters, then converts back.
/// Produces a logarithmic spiral bitcrushing distortion.
#[derive(Debug, Clone)]
pub struct SpiralBitcrushEffect;

impl SpiralBitcrushEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectApplication for SpiralBitcrushEffect {
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
        let domain_x = (values[1] + 0.001).floor().max(2.0) as f64;
        let domain_y = (domain_x * values[2] as f64).round();
        let zoom = values[3] as f64 * TAU;
        let rotation = values[4] as f64 * TAU;

        let domain_hypot = domain_x.hypot(domain_y);
        let domain_theta = domain_y.atan2(domain_x);

        let scale = domain_hypot / TAU;

        let x = input.x as f64;
        let y = input.y as f64;
        let z = input.z as f64;

        // Convert to log-polar
        let r = x.hypot(y);
        if r < 1e-10 {
            return input;
        }
        let log_r = r.ln();
        let theta = x.atan2(-y);

        // Log-polar coordinates with offsets
        let lp_x = theta - rotation;
        let lp_y = log_r - zoom;

        // Rotate by domain theta
        let cos_dt = domain_theta.cos();
        let sin_dt = domain_theta.sin();
        let rot_x = lp_x * cos_dt + lp_y * sin_dt;
        let rot_y = -lp_x * sin_dt + lp_y * cos_dt;

        // Scale and quantize
        let scaled_x = rot_x * scale;
        let scaled_y = rot_y * scale;
        let quant_x = scaled_x.round();
        let quant_y = scaled_y.round();

        // Unscale
        let unscaled_x = quant_x / scale;
        let unscaled_y = quant_y / scale;

        // Rotate back
        let out_lp_x = unscaled_x * cos_dt - unscaled_y * sin_dt;
        let out_lp_y = unscaled_x * sin_dt + unscaled_y * cos_dt;

        // Convert back from log-polar
        let out_log_r = out_lp_y + zoom;
        let out_theta = out_lp_x + rotation;
        let out_r = out_log_r.exp();

        let out_x = (out_r * out_theta.sin()) as f32;
        let out_y = (out_r * (-out_theta.cos())) as f32;

        // Quantize z in log space
        let out_z = if z.abs() > 1e-10 {
            let abs_z = z.abs();
            let log_z = abs_z.ln();
            let quantized_log_z = (log_z * scale).round() / scale;
            quantized_log_z.exp() as f32 * z.signum() as f32
        } else {
            input.z
        };

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
        "SpiralBitcrush"
    }
}
