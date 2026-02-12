use osci_core::{EffectApplication, Point};

/// BitCrush effect — quantises spatial coordinates to simulate bit-depth reduction.
///
/// Algorithm: `dequant * round(input * quant)` where the quantisation level is
/// derived from a power-based depth curve.
#[derive(Debug, Clone)]
pub struct BitCrush;

impl BitCrush {
    pub fn new() -> Self {
        Self
    }
}

impl EffectApplication for BitCrush {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        _sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        let effect_scale = values[0].clamp(0.0, 1.0);
        let value = values[1];

        let ranged_value = value * 0.78;
        let pow_value = (2.0_f32).powf(1.0 - ranged_value) - 1.0;
        let crush = pow_value * 12.0;
        let x = (2.0_f32).powf(crush);
        let quant = 0.5 * x;
        let dequant = 1.0 / quant;

        let output = Point::with_rgb(
            dequant * (input.x * quant).round(),
            dequant * (input.y * quant).round(),
            dequant * (input.z * quant).round(),
            input.r,
            input.g,
            input.b,
        );

        // Blend: (1 - effectScale) * input + effectScale * output
        // Using Point * f32 which preserves rgb, then Point + Point which adds all channels.
        // We need to handle colour correctly: both sides carry the same rgb so the linear
        // combination of colours must also be correct.
        let result = Point::with_rgb(
            (1.0 - effect_scale) * input.x + effect_scale * output.x,
            (1.0 - effect_scale) * input.y + effect_scale * output.y,
            (1.0 - effect_scale) * input.z + effect_scale * output.z,
            input.r,
            input.g,
            input.b,
        );

        result
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "BitCrush"
    }
}
