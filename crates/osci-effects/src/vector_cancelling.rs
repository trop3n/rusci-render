use osci_core::{EffectApplication, Point};

/// VectorCancelling effect — periodically inverts the XY coordinates of the
/// input, creating a visual cancellation pattern at a frequency derived from
/// the parameter value.
#[derive(Debug, Clone)]
pub struct VectorCancelling {
    last_index: usize,
    next_invert: f64,
}

impl VectorCancelling {
    pub fn new() -> Self {
        Self {
            last_index: 0,
            next_invert: 0.0,
        }
    }
}

impl EffectApplication for VectorCancelling {
    fn apply(
        &mut self,
        index: usize,
        mut input: Point,
        _external_input: Point,
        values: &[f32],
        _sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        let value = values[0];
        if value < 0.001 {
            return input;
        }

        let cancellation_frequency = 1.0 + 9.0 * value as f64;

        if index < self.last_index {
            self.next_invert = self.next_invert - self.last_index as f64 + cancellation_frequency;
        }
        self.last_index = index;

        if index >= self.next_invert as usize {
            self.next_invert += cancellation_frequency;
            input
        } else {
            input.scale(-1.0, -1.0, 1.0);
            input
        }
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Vector Cancelling"
    }
}
