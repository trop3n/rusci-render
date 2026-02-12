use osci_core::{EffectApplication, Point};

const MAX_DELAY: usize = 1_920_000;

/// Delay/echo effect.
///
/// Maintains a circular delay buffer and mixes an echo of the signal back in
/// with configurable decay and delay length.
#[derive(Debug, Clone)]
pub struct DelayEffect {
    delay_buffer: Vec<Point>,
    head: usize,
    position: usize,
    samples_since_last_delay: usize,
}

impl DelayEffect {
    pub fn new() -> Self {
        Self {
            delay_buffer: vec![Point::ZERO; MAX_DELAY],
            head: 0,
            position: 0,
            samples_since_last_delay: 0,
        }
    }
}

impl EffectApplication for DelayEffect {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        let decay = values[0];
        let decay_length = values[1];

        let delay_buffer_length = (sample_rate * decay_length) as usize;
        let buffer_size = self.delay_buffer.len();

        if self.head >= buffer_size {
            self.head -= buffer_size;
        }
        if self.position >= buffer_size {
            self.position -= buffer_size;
        }

        if self.samples_since_last_delay >= delay_buffer_length {
            self.samples_since_last_delay = 0;
            if self.head >= delay_buffer_length {
                self.position = self.head - delay_buffer_length;
            } else {
                self.position = buffer_size + self.head - delay_buffer_length;
            }
        }

        let echo = self.delay_buffer[self.position];

        let vector = Point::with_rgb(
            input.x + echo.x * decay,
            input.y + echo.y * decay,
            input.z + echo.z * decay,
            input.r,
            input.g,
            input.b,
        );

        self.delay_buffer[self.head] = vector;
        self.head += 1;
        self.position += 1;
        self.samples_since_last_delay += 1;

        vector
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Delay"
    }
}
