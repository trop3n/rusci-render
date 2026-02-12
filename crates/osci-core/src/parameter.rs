use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};

/// Smoothing speed constant (default for value changes).
pub const SMOOTHING_SPEED_CONSTANT: f32 = 0.3;
pub const SMOOTHING_SPEED_MIN: f32 = 0.00001;
/// Threshold below which we snap to target instead of smoothing.
pub const EFFECT_SNAP_THRESHOLD: f32 = 1e-4;

/// LFO waveform types, matching the C++ `osci::LfoType` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum LfoType {
    Static = 1,
    Sine = 2,
    Square = 3,
    Seesaw = 4,
    Triangle = 5,
    Sawtooth = 6,
    ReverseSawtooth = 7,
    Noise = 8,
}

impl LfoType {
    pub fn from_i32(val: i32) -> Self {
        match val {
            1 => LfoType::Static,
            2 => LfoType::Sine,
            3 => LfoType::Square,
            4 => LfoType::Seesaw,
            5 => LfoType::Triangle,
            6 => LfoType::Sawtooth,
            7 => LfoType::ReverseSawtooth,
            8 => LfoType::Noise,
            _ => LfoType::Static,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            LfoType::Static => "Static",
            LfoType::Sine => "Sine",
            LfoType::Square => "Square",
            LfoType::Seesaw => "Seesaw",
            LfoType::Triangle => "Triangle",
            LfoType::Sawtooth => "Sawtooth",
            LfoType::ReverseSawtooth => "Reverse Sawtooth",
            LfoType::Noise => "Noise",
        }
    }
}

/// Atomic f32 wrapper for lock-free audio-thread access.
#[derive(Debug)]
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    pub fn new(val: f32) -> Self {
        Self(AtomicU32::new(val.to_bits()))
    }

    pub fn load(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }

    pub fn store(&self, val: f32) {
        self.0.store(val.to_bits(), Ordering::Relaxed);
    }
}

impl Default for AtomicF32 {
    fn default() -> Self {
        Self::new(0.0)
    }
}

impl Clone for AtomicF32 {
    fn clone(&self) -> Self {
        Self::new(self.load())
    }
}

/// A single effect parameter with range, LFO modulation, and smoothing.
///
/// Mirrors the C++ `osci::EffectParameter`. Each parameter has:
/// - A value with min/max range
/// - Optional LFO modulation (type, rate, start/end percent)
/// - Per-sample smoothing
/// - Sidechain input option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EffectParameter {
    pub id: String,
    pub name: String,
    pub description: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub default_value: f32,
    pub step: f32,

    // LFO modulation
    pub lfo_type: LfoType,
    pub lfo_rate: f32,
    pub lfo_start_percent: f32,
    pub lfo_end_percent: f32,
    pub lfo_enabled: bool,

    // Smoothing
    pub smooth_value_change: f32,

    // Audio-thread state (not serialized directly)
    #[serde(skip)]
    pub phase: f32,
    #[serde(skip)]
    pub rng_state: u32,

    // Sidechain
    pub sidechain_enabled: bool,
}

impl EffectParameter {
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        id: impl Into<String>,
        value: f32,
        min: f32,
        max: f32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            value,
            min,
            max,
            default_value: value,
            step: 0.0001,
            lfo_type: LfoType::Static,
            lfo_rate: 1.0,
            lfo_start_percent: 0.0,
            lfo_end_percent: 100.0,
            lfo_enabled: true,
            smooth_value_change: SMOOTHING_SPEED_CONSTANT,
            phase: 0.0,
            rng_state: 0x12345678,
            sidechain_enabled: false,
        }
    }

    pub fn with_step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn with_lfo_default(mut self, lfo_type: LfoType, rate: f32) -> Self {
        self.lfo_type = lfo_type;
        self.lfo_rate = rate;
        self
    }

    pub fn without_lfo(mut self) -> Self {
        self.lfo_enabled = false;
        self
    }

    /// Get the normalized value in [0, 1].
    pub fn normalized_value(&self) -> f32 {
        if self.max == self.min {
            return 0.0;
        }
        ((self.value - self.min) / (self.max - self.min)).clamp(0.0, 1.0)
    }

    /// Set value from a normalized [0, 1] input.
    pub fn set_from_normalized(&mut self, normalized: f32) {
        let clamped = normalized.clamp(0.0, 1.0);
        self.value = self.min + clamped * (self.max - self.min);
    }

    /// Reset to default value and modulation settings.
    pub fn reset_to_default(&mut self) {
        self.value = self.default_value;
        self.smooth_value_change = SMOOTHING_SPEED_CONSTANT;
        self.lfo_type = LfoType::Static;
        self.lfo_rate = 1.0;
        self.lfo_start_percent = 0.0;
        self.lfo_end_percent = 100.0;
        self.sidechain_enabled = false;
        self.phase = 0.0;
        self.rng_state = 0x12345678;
    }

    /// Compute the LFO range in parameter units.
    pub fn lfo_range(&self) -> (f32, f32) {
        let range = self.max - self.min;
        let lfo_min = self.min + (self.lfo_start_percent / 100.0).clamp(0.0, 1.0) * range;
        let lfo_max = self.min + (self.lfo_end_percent / 100.0).clamp(0.0, 1.0) * range;
        (lfo_min, lfo_max)
    }
}

/// Animate a block of parameter values, computing per-sample values with
/// LFO modulation and smoothing.
///
/// This mirrors the C++ `Effect::animateValues()` method. It fills `output`
/// with the animated value for each sample in the block.
pub fn animate_parameter(
    param: &mut EffectParameter,
    output: &mut [f32],
    sample_rate: f32,
    current_value: &mut f32,
    volume_buffer: Option<&[f32]>,
) {
    let block_size = output.len();
    let range = param.max - param.min;

    if param.lfo_type == LfoType::Static {
        // Static path: smoothing towards target
        let smooth_raw = param.smooth_value_change;
        let instant = smooth_raw >= 1.0;
        let weight = if instant {
            1.0
        } else {
            let svc = smooth_raw.clamp(SMOOTHING_SPEED_MIN, 1.0);
            svc * (192000.0 / sample_rate) * 0.001
        };

        let use_sidechain = param.sidechain_enabled;
        let static_target = if use_sidechain { 0.0 } else { param.value };

        for i in 0..block_size {
            let target = if use_sidechain {
                let volume = volume_buffer
                    .and_then(|buf| buf.get(i).copied())
                    .unwrap_or(1.0);
                volume * range + param.min
            } else {
                static_target
            };

            if instant {
                *current_value = target;
            } else {
                let diff = (*current_value - target).abs();
                if diff < EFFECT_SNAP_THRESHOLD {
                    *current_value = target;
                } else {
                    *current_value += weight * (target - *current_value);
                }
            }
            output[i] = *current_value;
        }
    } else {
        // LFO path
        let (lfo_min, lfo_max) = param.lfo_range();
        let lfo_range = lfo_max - lfo_min;
        let phase_inc = if sample_rate > 0.0 { param.lfo_rate / sample_rate } else { 0.0 };

        let two_pi = std::f32::consts::TAU;
        let pi = std::f32::consts::PI;

        match param.lfo_type {
            LfoType::Noise => {
                for i in 0..block_size {
                    // xorshift32 PRNG
                    param.rng_state ^= param.rng_state << 13;
                    param.rng_state ^= param.rng_state >> 17;
                    param.rng_state ^= param.rng_state << 5;
                    let rnd = (param.rng_state & 0x00FFFFFF) as f32 / 16777215.0;
                    output[i] = rnd * lfo_range + lfo_min;
                }
            }
            _ => {
                // Phase ramp
                for i in 0..block_size {
                    param.phase += phase_inc;
                    if param.phase >= 1.0 {
                        param.phase -= 1.0;
                    }
                    output[i] = param.phase;
                }

                match param.lfo_type {
                    LfoType::Sine => {
                        for i in 0..block_size {
                            let p = output[i];
                            let s = (p * two_pi - pi).sin() * 0.5 + 0.5;
                            output[i] = s * lfo_range + lfo_min;
                        }
                    }
                    LfoType::Square => {
                        for i in 0..block_size {
                            output[i] = if output[i] < 0.5 { lfo_max } else { lfo_min };
                        }
                    }
                    LfoType::Seesaw => {
                        for i in 0..block_size {
                            let p = output[i];
                            let tri = if p < 0.5 { p * 2.0 } else { (1.0 - p) * 2.0 };
                            let x = tri.clamp(0.0, 1.0);
                            let soft = x * x * (3.0 - 2.0 * x); // smoothstep
                            output[i] = soft * lfo_range + lfo_min;
                        }
                    }
                    LfoType::Triangle => {
                        for i in 0..block_size {
                            let p = output[i];
                            let tri = 1.0 - (2.0 * p - 1.0).abs();
                            output[i] = tri * lfo_range + lfo_min;
                        }
                    }
                    LfoType::Sawtooth => {
                        for i in 0..block_size {
                            let p = output[i];
                            output[i] = p * lfo_range + lfo_min;
                        }
                    }
                    LfoType::ReverseSawtooth => {
                        for i in 0..block_size {
                            let p = output[i];
                            output[i] = (1.0 - p) * lfo_range + lfo_min;
                        }
                    }
                    _ => {
                        // Fallback: static value
                        for v in output.iter_mut() {
                            *v = param.value;
                        }
                    }
                }
            }
        }

        *current_value = output[block_size - 1];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lfo_type_roundtrip() {
        for i in 1..=8 {
            let t = LfoType::from_i32(i);
            assert_eq!(t as i32, i);
        }
    }

    #[test]
    fn test_parameter_normalize() {
        let p = EffectParameter::new("Test", "Test param", "test", 0.5, 0.0, 1.0);
        assert!((p.normalized_value() - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_animate_static() {
        let mut param = EffectParameter::new("Test", "Test", "test", 0.75, 0.0, 1.0);
        param.smooth_value_change = 1.0; // instant
        let mut output = vec![0.0f32; 128];
        let mut current = 0.0;
        animate_parameter(&mut param, &mut output, 44100.0, &mut current, None);
        for v in &output {
            assert!((*v - 0.75).abs() < 0.001);
        }
    }

    #[test]
    fn test_animate_sine_lfo() {
        let mut param = EffectParameter::new("Test", "Test", "test", 0.5, 0.0, 1.0);
        param.lfo_type = LfoType::Sine;
        param.lfo_rate = 1.0;
        let mut output = vec![0.0f32; 44100];
        let mut current = 0.5;
        animate_parameter(&mut param, &mut output, 44100.0, &mut current, None);
        // Sine should oscillate between 0 and 1
        let min = output.iter().cloned().fold(f32::MAX, f32::min);
        let max = output.iter().cloned().fold(f32::MIN, f32::max);
        assert!(min < 0.1);
        assert!(max > 0.9);
    }

    #[test]
    fn test_atomic_f32() {
        let a = AtomicF32::new(3.14);
        assert!((a.load() - 3.14).abs() < 0.001);
        a.store(2.71);
        assert!((a.load() - 2.71).abs() < 0.001);
    }
}
