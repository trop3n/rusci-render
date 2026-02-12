use osci_core::{EffectApplication, Point};

/// Volume effect — output gain scaling.
///
/// Multiplies the spatial coordinates by a gain factor. This is a system-level
/// effect typically placed at the end of the effect chain.
#[derive(Debug, Clone)]
pub struct VolumeEffect;

impl VolumeEffect {
    pub fn new() -> Self {
        Self
    }
}

impl EffectApplication for VolumeEffect {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        _sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        let gain = values[0];
        Point::with_rgb(
            input.x * gain,
            input.y * gain,
            input.z * gain,
            input.r,
            input.g,
            input.b,
        )
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Volume"
    }
}
