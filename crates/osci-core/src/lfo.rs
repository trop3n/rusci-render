use crate::parameter::LfoType;
use serde::{Deserialize, Serialize};

/// LFO state for a single parameter modulation source.
///
/// This is a convenience struct for standalone LFO computation
/// outside of the block-based `animate_parameter` system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LfoState {
    pub lfo_type: LfoType,
    pub rate: f32,
    pub phase: f32,
    pub start_percent: f32,
    pub end_percent: f32,
    pub rng_state: u32,
}

impl Default for LfoState {
    fn default() -> Self {
        Self {
            lfo_type: LfoType::Static,
            rate: 1.0,
            phase: 0.0,
            start_percent: 0.0,
            end_percent: 100.0,
            rng_state: 0x12345678,
        }
    }
}

impl LfoState {
    pub fn new(lfo_type: LfoType, rate: f32) -> Self {
        Self {
            lfo_type,
            rate,
            ..Default::default()
        }
    }

    /// Compute the LFO value for a given sample, advancing internal phase.
    ///
    /// Returns a value in [min_value, max_value] based on the LFO waveform.
    pub fn next_value(&mut self, min_value: f32, max_value: f32, sample_rate: f32) -> f32 {
        let (lfo_min, lfo_max) = self.compute_range(min_value, max_value);
        let lfo_range = lfo_max - lfo_min;

        match self.lfo_type {
            LfoType::Static => min_value, // No modulation
            LfoType::Noise => {
                self.rng_state ^= self.rng_state << 13;
                self.rng_state ^= self.rng_state >> 17;
                self.rng_state ^= self.rng_state << 5;
                let rnd = (self.rng_state & 0x00FFFFFF) as f32 / 16777215.0;
                rnd * lfo_range + lfo_min
            }
            _ => {
                // Advance phase
                if sample_rate > 0.0 {
                    self.phase += self.rate / sample_rate;
                    if self.phase >= 1.0 {
                        self.phase -= 1.0;
                    }
                }
                self.waveform_value(self.phase, lfo_min, lfo_max, lfo_range)
            }
        }
    }

    fn compute_range(&self, min_value: f32, max_value: f32) -> (f32, f32) {
        let range = max_value - min_value;
        let lfo_min = min_value + (self.start_percent / 100.0).clamp(0.0, 1.0) * range;
        let lfo_max = min_value + (self.end_percent / 100.0).clamp(0.0, 1.0) * range;
        (lfo_min, lfo_max)
    }

    fn waveform_value(&self, phase: f32, lfo_min: f32, lfo_max: f32, lfo_range: f32) -> f32 {
        let two_pi = std::f32::consts::TAU;
        let pi = std::f32::consts::PI;

        match self.lfo_type {
            LfoType::Sine => {
                let s = (phase * two_pi - pi).sin() * 0.5 + 0.5;
                s * lfo_range + lfo_min
            }
            LfoType::Square => {
                if phase < 0.5 { lfo_max } else { lfo_min }
            }
            LfoType::Seesaw => {
                let tri = if phase < 0.5 { phase * 2.0 } else { (1.0 - phase) * 2.0 };
                let x = tri.clamp(0.0, 1.0);
                let soft = x * x * (3.0 - 2.0 * x);
                soft * lfo_range + lfo_min
            }
            LfoType::Triangle => {
                let tri = 1.0 - (2.0 * phase - 1.0).abs();
                tri * lfo_range + lfo_min
            }
            LfoType::Sawtooth => {
                phase * lfo_range + lfo_min
            }
            LfoType::ReverseSawtooth => {
                (1.0 - phase) * lfo_range + lfo_min
            }
            _ => lfo_min,
        }
    }

    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.rng_state = 0x12345678;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sine_lfo_range() {
        let mut lfo = LfoState::new(LfoType::Sine, 1.0);
        let mut min_seen = f32::MAX;
        let mut max_seen = f32::MIN;

        for _ in 0..44100 {
            let v = lfo.next_value(0.0, 1.0, 44100.0);
            min_seen = min_seen.min(v);
            max_seen = max_seen.max(v);
        }

        assert!(min_seen < 0.05);
        assert!(max_seen > 0.95);
    }

    #[test]
    fn test_square_lfo() {
        let mut lfo = LfoState::new(LfoType::Square, 1.0);
        let v1 = lfo.next_value(0.0, 1.0, 44100.0);
        assert!((v1 - 1.0).abs() < 0.001); // First sample: phase near 0, so lfo_max
    }

    #[test]
    fn test_static_lfo() {
        let mut lfo = LfoState::new(LfoType::Static, 1.0);
        let v = lfo.next_value(0.0, 1.0, 44100.0);
        assert!((v).abs() < 0.001); // Returns min_value
    }
}
