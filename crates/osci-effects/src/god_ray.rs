use osci_core::{EffectApplication, Point};

/// GodRay effect — per-sample noise with directional bias.
///
/// Applies random brightness/scale modulation to each sample,
/// producing god-ray-like streaking effects. Uses a simple LCG
/// random number generator for deterministic noise.
#[derive(Debug, Clone)]
pub struct GodRayEffect {
    rng_state: u64,
}

impl GodRayEffect {
    pub fn new() -> Self {
        Self {
            rng_state: 123456789,
        }
    }

    /// Simple LCG random number generator returning a value in [0, 1).
    fn next_random(&mut self) -> f64 {
        // LCG parameters (Numerical Recipes)
        self.rng_state = self.rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        // Extract bits and convert to [0, 1)
        let bits = (self.rng_state >> 33) as f64;
        bits / (1u64 << 31) as f64
    }
}

impl EffectApplication for GodRayEffect {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        _sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        let noise_amp = values[0].max(0.0) as f64;
        let bias = values[1] as f64;
        let bias_exponent = 12.0_f64.powf(bias.abs());

        let mut noise = self.next_random();

        if bias > 0.0 {
            noise = noise.powf(bias_exponent);
        } else {
            noise = 1.0 - (1.0 - noise).powf(bias_exponent);
        }

        let scale = ((1.0 - noise_amp) + noise * noise_amp) as f32;

        Point::with_rgb(
            input.x * scale,
            input.y * scale,
            input.z * scale,
            input.r,
            input.g,
            input.b,
        )
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "GodRay"
    }
}
