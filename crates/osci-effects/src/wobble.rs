use osci_core::effect::PhaseAccumulator;
use osci_core::{EffectApplication, Point};

/// Wobble effect.
///
/// Adds a sinusoidal displacement to the input point. The wobble amplitude
/// is controlled by `values[0]` and the phase offset by `values[1]`.
#[derive(Debug, Clone)]
pub struct WobbleEffect {
    phase: PhaseAccumulator,
}

impl WobbleEffect {
    pub fn new() -> Self {
        Self {
            phase: PhaseAccumulator::new(),
        }
    }
}

impl EffectApplication for WobbleEffect {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        sample_rate: f32,
        frequency: f32,
    ) -> Point {
        let wobble_phase = values[1] as f64 * std::f64::consts::PI;
        let theta = self.phase.next_phase(frequency as f64, sample_rate as f64) + wobble_phase;
        let delta = 0.5 * values[0] * theta.sin() as f32;

        input + delta
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Wobble"
    }
}
