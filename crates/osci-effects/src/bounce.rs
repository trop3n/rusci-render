use osci_core::{EffectApplication, Point};
use std::f32::consts::{PI, TAU};

/// Bounce effect — 2D physics simulation with edge collision.
///
/// Simulates a bouncing point that reflects off the edges of a bounding
/// box. The input shape is scaled and translated to follow the bouncing
/// point's position.
#[derive(Debug, Clone)]
pub struct BounceEffect {
    position: Point,
    flip_x: bool,
    flip_y: bool,
}

impl BounceEffect {
    pub fn new() -> Self {
        Self {
            position: Point::ZERO,
            flip_x: false,
            flip_y: false,
        }
    }
}

impl EffectApplication for BounceEffect {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        let size = values[0].clamp(0.05, 1.0);
        let speed = values[1];
        let angle = values[2] * TAU - PI;

        let mut dir_x = angle.cos();
        let mut dir_y = angle.sin();

        if self.flip_x {
            dir_x = -dir_x;
        }
        if self.flip_y {
            dir_y = -dir_y;
        }

        let dt = 1.0 / sample_rate;
        self.position.x += dir_x * speed * dt;
        self.position.y += dir_y * speed * dt;

        let boundary = 1.0 - size;

        // Bounce at X boundaries
        if self.position.x > boundary {
            self.position.x = boundary;
            self.flip_x = !self.flip_x;
        } else if self.position.x < -boundary {
            self.position.x = -boundary;
            self.flip_x = !self.flip_x;
        }

        // Bounce at Y boundaries
        if self.position.y > boundary {
            self.position.y = boundary;
            self.flip_y = !self.flip_y;
        } else if self.position.y < -boundary {
            self.position.y = -boundary;
            self.flip_y = !self.flip_y;
        }

        let mut output = input * size + self.position;
        output.z = input.z;
        output.r = input.r;
        output.g = input.g;
        output.b = input.b;

        output
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Bounce"
    }
}
