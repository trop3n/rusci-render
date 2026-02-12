use crate::point::Point;
use std::f64::consts::PI;

/// Context provided to each effect during sample processing.
pub struct EffectContext {
    pub sample_rate: f32,
    pub frequency: f32,
}

/// The core trait for effect DSP implementations.
///
/// Mirrors the C++ `osci::EffectApplication` interface. Each effect
/// implementation processes one sample at a time, reading animated
/// parameter values from a slice.
pub trait EffectApplication: Send + Sync {
    /// Process a single sample. `index` is the sample index within the block.
    /// `input` is the current point, `values` are the animated parameter values.
    fn apply(
        &mut self,
        index: usize,
        input: Point,
        external_input: Point,
        values: &[f32],
        sample_rate: f32,
        frequency: f32,
    ) -> Point;

    /// Clone this effect application for per-voice instances.
    fn clone_effect(&self) -> Box<dyn EffectApplication>;

    /// Effect name for display.
    fn name(&self) -> &str;
}

/// Phase tracking for oscillating effects.
///
/// Matches the C++ `EffectApplication::nextPhase` / `resetPhase` methods.
#[derive(Debug, Clone)]
pub struct PhaseAccumulator {
    phase: f64,
}

impl Default for PhaseAccumulator {
    fn default() -> Self {
        Self { phase: -PI }
    }
}

impl PhaseAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.phase = -PI;
    }

    /// Advance phase by frequency/sample_rate and return the new phase.
    pub fn next_phase(&mut self, frequency: f64, sample_rate: f64) -> f64 {
        self.phase += 2.0 * PI * frequency / sample_rate;
        self.phase = wrap_angle(self.phase);
        self.phase
    }

    pub fn phase(&self) -> f64 {
        self.phase
    }
}

/// Wrap an angle to [-pi, pi].
pub fn wrap_angle(angle: f64) -> f64 {
    let two_pi = 2.0 * PI;
    ((angle + PI) % two_pi + two_pi) % two_pi - PI
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_angle() {
        let a = wrap_angle(4.0 * PI);
        assert!((a).abs() < 0.001);
    }

    #[test]
    fn test_phase_accumulator() {
        let mut phase = PhaseAccumulator::new();
        // At 1 Hz, 44100 sample rate, one full cycle = 44100 samples
        for _ in 0..44100 {
            phase.next_phase(1.0, 44100.0);
        }
        // After one full cycle, phase should be near start
        assert!((phase.phase() - (-PI)).abs() < 0.01);
    }
}
