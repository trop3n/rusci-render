use osci_core::effect::PhaseAccumulator;
use osci_core::{EffectApplication, Point};
use std::f64::consts::{PI, TAU};

/// Unfold effect — polar coordinate angular compression.
///
/// Compresses the angular range of the input into wedge-shaped segments,
/// effectively "unfolding" a shape into repeated angular slices.
#[derive(Debug, Clone)]
pub struct UnfoldEffect {
    phase: PhaseAccumulator,
}

impl UnfoldEffect {
    pub fn new() -> Self {
        Self {
            phase: PhaseAccumulator::new(),
        }
    }
}

impl EffectApplication for UnfoldEffect {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        sample_rate: f32,
        frequency: f32,
    ) -> Point {
        let segments = values[0].max(1.0) as f64;

        let full_segments = segments.floor();

        // Advance phase
        let phase_val = self.phase.next_phase(
            frequency as f64 / (full_segments + 1.0),
            sample_rate as f64,
        );
        // Normalize phase from [-PI, PI] to [0, 1]
        let normalized_phase = (phase_val + PI) / TAU;

        let current_segment_float = normalized_phase * segments;
        let current_segment = current_segment_float.floor();

        // Convert input to polar
        let r = ((input.x as f64) * (input.x as f64)
            + (input.y as f64) * (input.y as f64))
            .sqrt();
        let theta = (input.y as f64).atan2(input.x as f64);

        // Wedge angle for each segment
        let wedge_angle = TAU / segments;

        // Map theta into current segment
        let segment_start = current_segment * wedge_angle - PI;
        let mut local_theta = theta - segment_start;

        // Wrap local_theta to [0, wedge_angle]
        local_theta = ((local_theta % wedge_angle) + wedge_angle) % wedge_angle;

        // Map back to full circle range
        let out_theta = local_theta * segments - PI;

        // Convert back to cartesian
        let out_x = r * out_theta.cos();
        let out_y = r * out_theta.sin();

        Point::with_rgb(
            out_x as f32,
            out_y as f32,
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
        "Unfold"
    }
}
