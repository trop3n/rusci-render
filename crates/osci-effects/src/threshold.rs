use osci_core::{EffectApplication, Point};

/// Threshold effect — hard clipping limiter.
///
/// Clamps the spatial coordinates to the range `[-level, level]`. This is a
/// system-level effect typically placed at the end of the effect chain to
/// prevent the output from exceeding the display bounds.
#[derive(Debug, Clone)]
pub struct ThresholdEffect;

impl ThresholdEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectApplication for ThresholdEffect {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        _sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        let level = values[0].max(0.0);
        Point::with_rgb(
            input.x.clamp(-level, level),
            input.y.clamp(-level, level),
            input.z.clamp(-level, level),
            input.r,
            input.g,
            input.b,
        )
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Threshold"
    }
}
