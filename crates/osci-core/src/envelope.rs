use serde::{Deserialize, Serialize};

/// Curve type for envelope segments, matching the C++ `EnvCurve::CurveType`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EnvCurveType {
    Empty,
    Numerical(f32),
    Step,
    Linear,
    Exponential,
    Sine,
    Welch,
}

/// A single envelope curve specification.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EnvCurve {
    pub curve_type: EnvCurveType,
}

impl EnvCurve {
    pub fn linear() -> Self {
        Self { curve_type: EnvCurveType::Linear }
    }

    pub fn numerical(curve: f32) -> Self {
        Self { curve_type: EnvCurveType::Numerical(curve) }
    }

    pub fn step() -> Self {
        Self { curve_type: EnvCurveType::Step }
    }

    pub fn exponential() -> Self {
        Self { curve_type: EnvCurveType::Exponential }
    }

    pub fn sine() -> Self {
        Self { curve_type: EnvCurveType::Sine }
    }

    pub fn welch() -> Self {
        Self { curve_type: EnvCurveType::Welch }
    }
}

impl Default for EnvCurve {
    fn default() -> Self {
        Self::linear()
    }
}

impl From<f32> for EnvCurve {
    fn from(curve: f32) -> Self {
        Self::numerical(curve)
    }
}

/// A segmented envelope specification (ADSR, ASR, perc, etc.).
///
/// Ported from the UGEN++ `Env` class used in osci-render.
/// Supports multiple segment types with different curve shapes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Env {
    pub levels: Vec<f64>,
    pub times: Vec<f64>,
    pub curves: Vec<EnvCurve>,
    pub release_node: i32,
    pub loop_node: i32,
}

impl Env {
    /// Create a new envelope with the given levels, times, and curves.
    pub fn new(
        levels: Vec<f64>,
        times: Vec<f64>,
        curves: Vec<EnvCurve>,
        release_node: i32,
        loop_node: i32,
    ) -> Self {
        Self { levels, times, curves, release_node, loop_node }
    }

    /// Standard ADSR envelope.
    ///
    /// Levels: [0, level, level*sustain_level, 0]
    /// Times: [attack, decay, release]
    /// Release node at index 2 (sustains until note off).
    pub fn adsr(
        attack_time: f64,
        decay_time: f64,
        sustain_level: f64,
        release_time: f64,
        level: f64,
        curve: f32,
    ) -> Self {
        Self::new(
            vec![0.0, level, level * sustain_level, 0.0],
            vec![attack_time, decay_time, release_time],
            vec![EnvCurve::numerical(curve); 3],
            2,
            -1,
        )
    }

    /// Attack-Sustain-Release envelope.
    pub fn asr(
        attack_time: f64,
        sustain_level: f64,
        release_time: f64,
        level: f64,
        curve: f32,
    ) -> Self {
        Self::new(
            vec![0.0, level * sustain_level, 0.0],
            vec![attack_time, release_time],
            vec![EnvCurve::numerical(curve); 2],
            1,
            -1,
        )
    }

    /// Percussive envelope (attack-release, no sustain).
    pub fn perc(attack_time: f64, release_time: f64, level: f64, curve: f32) -> Self {
        Self::new(
            vec![0.0, level, 0.0],
            vec![attack_time, release_time],
            vec![EnvCurve::numerical(curve); 2],
            -1,
            -1,
        )
    }

    /// Linear-in-envelope: attack → sustain → release.
    pub fn linen(
        attack_time: f64,
        sustain_time: f64,
        release_time: f64,
        sustain_level: f64,
    ) -> Self {
        Self::new(
            vec![0.0, sustain_level, sustain_level, 0.0],
            vec![attack_time, sustain_time, release_time],
            vec![EnvCurve::linear(); 3],
            -1,
            -1,
        )
    }

    /// Triangle envelope.
    pub fn triangle(duration: f64, level: f64) -> Self {
        let half = duration * 0.5;
        Self::new(
            vec![0.0, level, 0.0],
            vec![half, half],
            vec![EnvCurve::linear(); 2],
            -1,
            -1,
        )
    }

    /// Sine-shaped envelope.
    pub fn sine_env(duration: f64, level: f64) -> Self {
        let half = duration * 0.5;
        Self::new(
            vec![0.0, level, 0.0],
            vec![half, half],
            vec![EnvCurve::sine(); 2],
            -1,
            -1,
        )
    }

    /// Total duration of the envelope.
    pub fn duration(&self) -> f64 {
        self.times.iter().sum()
    }

    /// Scale all levels by a constant.
    pub fn level_scale(&self, scale: f64) -> Self {
        Self::new(
            self.levels.iter().map(|l| l * scale).collect(),
            self.times.clone(),
            self.curves.clone(),
            self.release_node,
            self.loop_node,
        )
    }

    /// Offset all levels by a constant.
    pub fn level_bias(&self, bias: f64) -> Self {
        Self::new(
            self.levels.iter().map(|l| l + bias).collect(),
            self.times.clone(),
            self.curves.clone(),
            self.release_node,
            self.loop_node,
        )
    }

    /// Scale all times by a constant.
    pub fn time_scale(&self, scale: f64) -> Self {
        Self::new(
            self.levels.clone(),
            self.times.iter().map(|t| t * scale).collect(),
            self.curves.clone(),
            self.release_node,
            self.loop_node,
        )
    }

    /// Look up the envelope value at a given time.
    ///
    /// Ignores loop_node and release_node — treats the envelope as a
    /// fixed-duration shape. This matches the C++ `Env::lookup()`.
    pub fn lookup(&self, time: f32) -> f32 {
        let num_times = self.times.len();
        let num_levels = self.levels.len();

        if num_levels < 1 {
            return 0.0;
        }
        if time <= 0.0 || num_times == 0 {
            return self.levels[0] as f32;
        }

        let mut last_time: f32 = 0.0;
        let mut stage_time: f32 = 0.0;
        let mut stage_index: usize = 0;

        while stage_time < time && stage_index < num_times {
            last_time = stage_time;
            stage_time += self.times[stage_index] as f32;
            stage_index += 1;
        }

        if stage_index > num_times {
            return self.levels[num_levels - 1] as f32;
        }

        let level0 = self.levels[stage_index - 1] as f32;
        let level1 = self.levels[stage_index] as f32;

        let curve_index = (stage_index - 1) % self.curves.len().max(1);
        let curve = &self.curves[curve_index];

        if (last_time - stage_time) == 0.0 {
            return level1;
        }

        match curve.curve_type {
            EnvCurveType::Linear => {
                linlin(time, last_time, stage_time, level0, level1)
            }
            EnvCurveType::Numerical(curve_value) => {
                if curve_value.abs() <= 0.001 {
                    linlin(time, last_time, stage_time, level0, level1)
                } else {
                    let pos = (time - last_time) / (stage_time - last_time);
                    let denom = 1.0 - (curve_value).exp();
                    let numer = 1.0 - (pos * curve_value).exp();
                    level0 + (level1 - level0) * (numer / denom)
                }
            }
            EnvCurveType::Sine => {
                linsin(time, last_time, stage_time, level0, level1)
            }
            EnvCurveType::Exponential => {
                linexp(time, last_time, stage_time, level0, level1)
            }
            EnvCurveType::Welch => {
                linwelch(time, last_time, stage_time, level0, level1)
            }
            EnvCurveType::Step | EnvCurveType::Empty => level1,
        }
    }
}

/// Linear interpolation.
fn linlin(input: f32, in_low: f32, in_high: f32, out_low: f32, out_high: f32) -> f32 {
    let in_range = in_high - in_low;
    let out_range = out_high - out_low;
    (input - in_low) * out_range / in_range + out_low
}

/// Linear-to-exponential mapping.
fn linexp(input: f32, in_low: f32, in_high: f32, out_low: f32, out_high: f32) -> f32 {
    let out_ratio = out_high / out_low;
    let recip_in_range = 1.0 / (in_high - in_low);
    let neg_in_low_over = recip_in_range * -in_low;
    out_low * out_ratio.powf(input * recip_in_range + neg_in_low_over)
}

/// Linear-to-sine mapping.
fn linsin(input: f32, in_low: f32, in_high: f32, out_low: f32, out_high: f32) -> f32 {
    let in_range = in_high - in_low;
    let out_range = out_high - out_low;
    let in_phase = (input - in_low) * std::f32::consts::PI / in_range + std::f32::consts::PI;
    let cos_in_phase = in_phase.cos() * 0.5 + 0.5;
    cos_in_phase * out_range + out_low
}

/// Linear-to-Welch mapping.
fn linwelch(input: f32, in_low: f32, in_high: f32, out_low: f32, out_high: f32) -> f32 {
    let in_range = in_high - in_low;
    let out_range = out_high - out_low;
    let in_pos = (input - in_low) / in_range;
    let half_pi = std::f32::consts::FRAC_PI_2;

    if out_low < out_high {
        out_low + out_range * (half_pi * in_pos).sin()
    } else {
        out_high - out_range * (half_pi - half_pi * in_pos).sin()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adsr_creation() {
        let env = Env::adsr(0.01, 0.3, 0.5, 1.0, 1.0, -4.0);
        assert_eq!(env.levels.len(), 4);
        assert_eq!(env.times.len(), 3);
        assert_eq!(env.release_node, 2);
    }

    #[test]
    fn test_adsr_lookup() {
        let env = Env::adsr(0.01, 0.3, 0.5, 1.0, 1.0, -4.0);
        // At time 0, level should be 0
        assert!((env.lookup(0.0)).abs() < 0.001);
        // At end of attack, level should approach 1.0
        let at_attack = env.lookup(0.01);
        assert!(at_attack > 0.9);
    }

    #[test]
    fn test_linlin() {
        let result = linlin(0.5, 0.0, 1.0, 0.0, 10.0);
        assert!((result - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_duration() {
        let env = Env::adsr(0.01, 0.3, 0.5, 1.0, 1.0, -4.0);
        assert!((env.duration() - 1.31).abs() < 0.001);
    }

    #[test]
    fn test_level_scale() {
        let env = Env::adsr(0.01, 0.3, 0.5, 1.0, 1.0, -4.0);
        let scaled = env.level_scale(2.0);
        assert!((scaled.levels[1] - 2.0).abs() < 0.001);
    }
}
