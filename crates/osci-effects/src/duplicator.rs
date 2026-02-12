use osci_core::{EffectApplication, Point};
use std::f64::consts::TAU;

/// Duplicator effect.
///
/// Creates multiple copies of the input shape spread in a circle around the
/// origin. `values[0]` is the number of copies, `values[1]` controls the
/// radial spread, and `values[2]` sets an angular offset.
#[derive(Debug, Clone)]
pub struct DuplicatorEffect {
    frame_phase: f64,
}

impl DuplicatorEffect {
    pub fn new() -> Self {
        Self {
            frame_phase: 0.0,
        }
    }
}

impl EffectApplication for DuplicatorEffect {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        sample_rate: f32,
        frequency: f32,
    ) -> Point {
        let copies = values[0].max(1.0);
        let spread = values[1].clamp(0.0, 1.0);
        let angle_offset = values[2] as f64 * TAU;

        let theta = (self.frame_phase * copies as f64).floor() / copies as f64 * TAU + angle_offset;
        let offset = Point::new(theta.cos() as f32, theta.sin() as f32, 0.0);

        let freq_divisor = (copies as f64 - 1e-3).ceil();
        self.frame_phase += frequency as f64 / freq_divisor / sample_rate as f64;
        // Wrap to [0, 1)
        if self.frame_phase >= 1.0 {
            self.frame_phase -= self.frame_phase.floor();
        }

        (1.0 - spread) * input + spread * offset
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Duplicator"
    }
}
