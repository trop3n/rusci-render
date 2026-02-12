use osci_core::{EffectApplication, Point};

/// Translate effect — offsets the input point by the parameter values along
/// each axis.
#[derive(Debug, Clone)]
pub struct Translate;

impl Translate {
    pub fn new() -> Self {
        Self
    }
}

impl EffectApplication for Translate {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        _sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        // Point::new sets r=g=b=z, so we use with_rgb to preserve colour.
        input + Point::with_rgb(values[0], values[1], values[2], 0.0, 0.0, 0.0)
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Translate"
    }
}
