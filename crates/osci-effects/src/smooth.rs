use osci_core::{EffectApplication, Point};

/// Smooth (low-pass EMA) effect.
///
/// Applies an exponential moving average to smooth the input signal.
/// The smoothing weight is derived from `values[0]` using a logarithmic curve,
/// then adjusted for sample rate so the behavior is consistent across rates.
#[derive(Debug, Clone)]
pub struct SmoothEffect {
    avg: Point,
}

impl SmoothEffect {
    pub fn new() -> Self {
        Self {
            avg: Point::ZERO,
        }
    }
}

impl EffectApplication for SmoothEffect {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        let weight = values[0].max(0.00001) * 0.95;
        let strength: f64 = 10.0;
        let weight = ((strength * weight as f64 + 1.0).ln() / (strength + 1.0).ln()) as f32;
        let weight = weight.powf(48000.0 / sample_rate);

        self.avg = weight * self.avg + (1.0 - weight) * input;

        self.avg
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Smooth"
    }
}
