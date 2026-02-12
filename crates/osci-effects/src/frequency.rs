use osci_core::{EffectApplication, Point};

/// Frequency effect — shape drawing rate control.
///
/// This is a system-level "effect" that doesn't transform points directly.
/// Instead, its parameter value (Hz) is read by the voice/renderer to control
/// the rate at which shapes are drawn. As an EffectApplication it passes
/// input through unchanged; the actual frequency adjustment happens at the
/// voice level.
#[derive(Debug, Clone)]
pub struct FrequencyEffect;

impl FrequencyEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectApplication for FrequencyEffect {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        _values: &[f32],
        _sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        // Pass-through: the frequency value is consumed by the voice/renderer,
        // not by the per-sample effect pipeline.
        input
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Frequency"
    }
}
