use osci_core::{EffectApplication, Point};

/// Perspective effect — 3D perspective projection via a pinhole camera model.
///
/// Places a virtual camera along the Z axis at a distance derived from the
/// field-of-view parameter. Points are projected through a frustum with
/// near/far clipping, producing a 2D perspective view.
#[derive(Debug, Clone)]
pub struct PerspectiveEffect {
    near: f32,
    far: f32,
}

impl PerspectiveEffect {
    pub fn new() -> Self {
        Self {
            near: 0.001,
            far: 100.0,
        }
    }
}

impl EffectApplication for PerspectiveEffect {
    fn apply(
        &mut self,
        _index: usize,
        input: Point,
        _external_input: Point,
        values: &[f32],
        _sample_rate: f32,
        _frequency: f32,
    ) -> Point {
        let effect_scale = values[0];
        let fov_degrees = values[1].clamp(1.5, 179.0);
        let fov = fov_degrees.to_radians();

        let tang = (fov * 0.5).tan();
        let focal_length = 1.0 / tang;

        // Place camera so FOV is tangent to unit sphere
        let cam_z = -1.0 / (0.5 * fov).sin();

        // Transform to camera space (translate by -camera position)
        let px = input.x;
        let py = input.y;
        let pz = input.z - cam_z;

        // Clip to frustum
        let cz = pz.clamp(self.near, self.far);
        let aux_y = (cz * tang).abs();
        let cy = py.clamp(-aux_y, aux_y);
        let aux_x = aux_y; // ratio = 1
        let cx = px.clamp(-aux_x, aux_x);

        // Project
        let proj_x = cx * focal_length / cz;
        let proj_y = cy * focal_length / cz;

        Point::with_rgb(
            (1.0 - effect_scale) * input.x + effect_scale * proj_x,
            (1.0 - effect_scale) * input.y + effect_scale * proj_y,
            0.0,
            input.r,
            input.g,
            input.b,
        )
    }

    fn clone_effect(&self) -> Box<dyn EffectApplication> {
        Box::new(self.clone())
    }

    fn name(&self) -> &str {
        "Perspective"
    }
}
