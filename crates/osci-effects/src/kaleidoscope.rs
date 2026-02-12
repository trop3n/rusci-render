use osci_core::{EffectApplication, Point};
use std::f64::consts::TAU;

/// Kaleidoscope effect — polar segment clipping with mirroring.
///
/// Divides the plane into angular segments and mirrors/clips the input
/// to create kaleidoscope-like symmetry patterns.
#[derive(Debug, Clone)]
pub struct KaleidoscopeEffect {
    frame_phase: f64,
}

impl KaleidoscopeEffect {
    pub fn new() -> Self {
        Self {
            frame_phase: 0.0,
        }
    }
}

impl EffectApplication for KaleidoscopeEffect {
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
        let mirror = values[1] as f64;
        let spread = values[2].clamp(0.0, 1.0) as f64;
        let clip = values[3].clamp(0.0, 1.0) as f64;

        // Rotate input 90 degrees clockwise: (y, -x, z)
        let rotated = Point::with_rgb(
            input.y,
            -input.x,
            input.z,
            input.r,
            input.g,
            input.b,
        );

        // Apply spread: blend between rotated input and its projection onto the x axis
        // x_axis projection = (rotated.x, 0, rotated.z)
        // output = (1-spread)*rotated + spread*(rotated.x, 0, rotated.z)
        let mut output = Point::with_rgb(
            rotated.x,
            ((1.0 - spread) as f32) * rotated.y,
            rotated.z,
            rotated.r,
            rotated.g,
            rotated.b,
        );

        // Determine which segment we are in based on frame_phase
        let current_segment = (self.frame_phase * segments).floor();
        let is_odd = (current_segment as i64) % 2 != 0;

        // Mirror y for odd segments
        if is_odd && mirror > 0.5 {
            output.y = -output.y;
        }

        // Clip to radial segment using plane normals
        let segment_angle = TAU / segments;
        let half_angle = segment_angle * 0.5;

        if clip > 0.0 {
            // Upper clipping plane normal
            let normal_upper_x = (-half_angle).sin() as f32;
            let normal_upper_y = (half_angle).cos() as f32;

            // Lower clipping plane normal
            let normal_lower_x = (half_angle).sin() as f32;
            let normal_lower_y = (half_angle).cos() as f32;

            let dot_upper = output.x * normal_upper_x + output.y * normal_upper_y;
            let dot_lower = output.x * normal_lower_x + output.y * normal_lower_y;

            if dot_upper < 0.0 {
                let scale = clip as f32;
                output.x -= dot_upper * normal_upper_x * scale;
                output.y -= dot_upper * normal_upper_y * scale;
            }
            if dot_lower < 0.0 {
                let scale = clip as f32;
                output.x -= dot_lower * normal_lower_x * scale;
                output.y -= dot_lower * normal_lower_y * scale;
            }
        }

        // Rotate to actual segment position
        let segment_rotation = (current_segment * segment_angle) as f32;
        let cos_r = segment_rotation.cos();
        let sin_r = segment_rotation.sin();
        let rx = output.x * cos_r - output.y * sin_r;
        let ry = output.x * sin_r + output.y * cos_r;
        output.x = rx;
        output.y = ry;

        // Advance frame phase
        let freq_divisor = (segments - 1e-3).ceil();
        self.frame_phase += frequency as f64 / freq_divisor / sample_rate as f64;
        if self.frame_phase >= 1.0 {
            self.frame_phase -= self.frame_phase.floor();
        }

        output
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Kaleidoscope"
    }
}
