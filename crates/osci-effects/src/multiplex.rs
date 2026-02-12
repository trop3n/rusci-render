use osci_core::effect::PhaseAccumulator;
use osci_core::{EffectApplication, Point};
use std::f64::consts::{PI, TAU};

const MAX_DELAY: usize = 1_920_000;

/// Multiplex effect — grid tessellation with buffered delay sampling.
///
/// Tiles the input shape across a 3D grid of configurable dimensions,
/// using a delay buffer to sample different grid positions at different
/// phases.
#[derive(Debug, Clone)]
pub struct MultiplexEffect {
    phase: PhaseAccumulator,
    buffer: Vec<Point>,
    head: usize,
}

impl MultiplexEffect {
    pub fn new() -> Self {
        Self {
            phase: PhaseAccumulator::new(),
            buffer: vec![Point::ZERO; MAX_DELAY],
            head: 0,
        }
    }
}

/// Multiplex helper: maps a point into a specific grid cell.
///
/// `grid` is the (gfx, gfy, gfz) grid dimensions (floored, >= 1).
/// `position` is the flat cell index (float).
/// `point` is the input point, which is scaled and repositioned.
fn multiplex(mut point: Point, grid_x: f64, grid_y: f64, grid_z: f64, position: f64) -> Point {
    let unit_x = 1.0 / grid_x;
    let unit_y = 1.0 / grid_y;
    let unit_z = 1.0 / grid_z;

    point.x *= unit_x as f32;
    point.y *= unit_y as f32;
    point.z *= unit_z as f32;

    point.x = -point.x;
    point.y = -point.y;

    let x_pos = ((position % grid_x) + grid_x) % grid_x;
    let y_pos = (((position / grid_x).floor() % grid_y) + grid_y) % grid_y;
    let z_pos = (((position / (grid_x * grid_y)).floor() % grid_z) + grid_z) % grid_z;

    point.x -= ((grid_x - 1.0) / grid_x) as f32;
    point.y += ((grid_y - 1.0) / grid_y) as f32;
    point.z += ((grid_z - 1.0) / grid_z) as f32;

    point.x += (x_pos * 2.0 * unit_x) as f32;
    point.y -= (y_pos * 2.0 * unit_y) as f32;
    point.z -= (z_pos * 2.0 * unit_z) as f32;

    point
}

impl EffectApplication for MultiplexEffect {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        sample_rate: f32,
        frequency: f32,
    ) -> Point {
        let grid_x = values[0];
        let grid_y = values[1];
        let grid_z = values[2];
        let interpolation = values[3];
        let grid_delay = values[4];

        let gfx = (grid_x + 1e-3).floor().max(1.0) as f64;
        let gfy = (grid_y + 1e-3).floor().max(1.0) as f64;
        let gfz = (grid_z + 1e-3).floor().max(1.0) as f64;

        let total_positions = gfx * gfy * gfz;

        let phase_val = self.phase.next_phase(
            frequency as f64 / total_positions,
            sample_rate as f64,
        );
        // Normalize phase from [-PI, PI] to [0, 1]
        let normalized_phase = (phase_val + PI) / TAU;

        let position = normalized_phase * total_positions;
        let delay_position = position.floor() / total_positions;

        // Store input in buffer
        let buffer_size = self.buffer.len();
        if self.head >= buffer_size {
            self.head = 0;
        }
        self.buffer[self.head] = input;

        // Calculate delayed index
        let delay_samples = (delay_position * grid_delay as f64 * sample_rate as f64) as i64;
        let mut delayed_index = self.head as i64 - delay_samples;
        while delayed_index < 0 {
            delayed_index += buffer_size as i64;
        }
        while delayed_index >= buffer_size as i64 {
            delayed_index -= buffer_size as i64;
        }

        let delayed_point = self.buffer[delayed_index as usize];

        // Current grid level (floored position)
        let current_pos = position.floor();
        let current = multiplex(delayed_point, gfx, gfy, gfz, current_pos);

        // Next grid level
        let next_pos = current_pos + 1.0;
        let next = multiplex(delayed_point, gfx, gfy, gfz, next_pos);

        // Interpolate between current and next grid level
        let frac = (position - current_pos) as f32;
        let interp_amount = frac * interpolation;

        let result = Point::with_rgb(
            current.x + (next.x - current.x) * interp_amount,
            current.y + (next.y - current.y) * interp_amount,
            current.z + (next.z - current.z) * interp_amount,
            delayed_point.r,
            delayed_point.g,
            delayed_point.b,
        );

        self.head += 1;

        result
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Multiplex"
    }
}
