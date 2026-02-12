use osci_core::{EffectApplication, Point};

/// Scale effect.
///
/// Multiplies the input point's spatial coordinates by per-axis scale factors
/// taken from `values[0..3]`. Color channels are preserved.
#[derive(Debug, Clone)]
pub struct ScaleEffect;

impl ScaleEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectApplication for ScaleEffect {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        _sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        input * Point::new(values[0], values[1], values[2])
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Scale"
    }
}
