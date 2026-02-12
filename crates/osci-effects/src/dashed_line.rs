use osci_core::{EffectApplication, Point};

const MAX_BUFFER: usize = 192_000;

/// DashedLine effect.
///
/// Segments the drawn waveform into dashes by sampling back into a circular
/// buffer. `dash_count` controls how many dashes per cycle, `dash_offset`
/// shifts them, and `dash_coverage` controls what fraction of each dash is
/// visible.
#[derive(Debug, Clone)]
pub struct DashedLineEffect {
    buffer: Vec<Point>,
    buffer_index: usize,
    frame_phase: f64,
}

impl DashedLineEffect {
    pub fn new() -> Self {
        Self {
            buffer: vec![Point::ZERO; MAX_BUFFER],
            buffer_index: 0,
            frame_phase: 0.0,
        }
    }
}

impl EffectApplication for DashedLineEffect {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        sample_rate: f32,
        frequency: f32,
    ) -> Point {
        let dash_count = values[0].max(1.0);
        let mut i = 1;
        let dash_offset = values[i];
        i += 1;
        let dash_coverage = values[i].clamp(0.0, 1.0);

        let dash_length_samples = (sample_rate as f64 / frequency as f64) / dash_count as f64;

        let mut dash_phase = self.frame_phase * dash_count as f64 - dash_offset as f64;
        dash_phase -= dash_phase.floor();

        let buffer_size = self.buffer.len();
        self.buffer[self.buffer_index] = input;

        let mut sample_pos = self.buffer_index as f64
            - dash_length_samples * dash_phase * (1.0 - dash_coverage as f64);

        // Wrap sample_pos to [0, buffer_size)
        while sample_pos < 0.0 {
            sample_pos += buffer_size as f64;
        }
        while sample_pos >= buffer_size as f64 {
            sample_pos -= buffer_size as f64;
        }

        let floor_idx = sample_pos.floor() as usize % buffer_size;
        let ceil_idx = (floor_idx + 1) % buffer_size;
        let frac = (sample_pos - sample_pos.floor()) as f32;

        let p0 = self.buffer[floor_idx];
        let p1 = self.buffer[ceil_idx];
        let result = Point::with_rgb(
            p0.x + (p1.x - p0.x) * frac,
            p0.y + (p1.y - p0.y) * frac,
            p0.z + (p1.z - p0.z) * frac,
            p0.r + (p1.r - p0.r) * frac,
            p0.g + (p1.g - p0.g) * frac,
            p0.b + (p1.b - p0.b) * frac,
        );

        self.buffer_index += 1;
        if self.buffer_index >= buffer_size {
            self.buffer_index = 0;
        }

        self.frame_phase += frequency as f64 / sample_rate as f64;
        if self.frame_phase >= 1.0 {
            self.frame_phase -= 1.0;
        }

        result
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "DashedLine"
    }
}

/// Trace effect.
///
/// A specialization of the dashed-line approach with a single dash
/// (`dash_count = 1`). Parameter indices start at 0 instead of 1.
#[derive(Debug, Clone)]
pub struct TraceEffect {
    buffer: Vec<Point>,
    buffer_index: usize,
    frame_phase: f64,
}

impl TraceEffect {
    pub fn new() -> Self {
        Self {
            buffer: vec![Point::ZERO; MAX_BUFFER],
            buffer_index: 0,
            frame_phase: 0.0,
        }
    }
}

impl EffectApplication for TraceEffect {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        sample_rate: f32,
        frequency: f32,
    ) -> Point {
        let dash_count = 1.0_f64;
        let mut i = 0;
        let dash_offset = values[i];
        i += 1;
        let dash_coverage = values[i].clamp(0.0, 1.0);

        let dash_length_samples = (sample_rate as f64 / frequency as f64) / dash_count;

        let mut dash_phase = self.frame_phase * dash_count - dash_offset as f64;
        dash_phase -= dash_phase.floor();

        let buffer_size = self.buffer.len();
        self.buffer[self.buffer_index] = input;

        let mut sample_pos = self.buffer_index as f64
            - dash_length_samples * dash_phase * (1.0 - dash_coverage as f64);

        // Wrap sample_pos to [0, buffer_size)
        while sample_pos < 0.0 {
            sample_pos += buffer_size as f64;
        }
        while sample_pos >= buffer_size as f64 {
            sample_pos -= buffer_size as f64;
        }

        let floor_idx = sample_pos.floor() as usize % buffer_size;
        let ceil_idx = (floor_idx + 1) % buffer_size;
        let frac = (sample_pos - sample_pos.floor()) as f32;

        let p0 = self.buffer[floor_idx];
        let p1 = self.buffer[ceil_idx];
        let result = Point::with_rgb(
            p0.x + (p1.x - p0.x) * frac,
            p0.y + (p1.y - p0.y) * frac,
            p0.z + (p1.z - p0.z) * frac,
            p0.r + (p1.r - p0.r) * frac,
            p0.g + (p1.g - p0.g) * frac,
            p0.b + (p1.b - p0.b) * frac,
        );

        self.buffer_index += 1;
        if self.buffer_index >= buffer_size {
            self.buffer_index = 0;
        }

        self.frame_phase += frequency as f64 / sample_rate as f64;
        if self.frame_phase >= 1.0 {
            self.frame_phase -= 1.0;
        }

        result
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Trace"
    }
}
