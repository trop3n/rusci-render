/// Visual parameters for the oscilloscope renderer.
#[derive(Clone)]
pub struct VisualiserSettings {
    /// Beam focus (Gaussian sigma in UV space). Smaller = sharper. Range: 0.001..0.02
    pub focus: f32,
    /// Beam intensity multiplier. Range: 0.1..5.0
    pub intensity: f32,
    /// Phosphor persistence (half-life in frames at 60fps). Range: 0.0..1.0
    pub persistence: f32,
    /// Afterglow color retention. Range: 0.0..1.0
    pub afterglow: f32,
    /// Tight bloom (glow) amount. Range: 0.0..2.0
    pub glow_amount: f32,
    /// Wide scatter bloom amount. Range: 0.0..2.0
    pub scatter_amount: f32,
    /// Beam color [r, g, b]. Range: 0.0..1.0 each
    pub color: [f32; 3],
    /// Tone mapping exposure. Range: 0.5..5.0
    pub exposure: f32,
    /// Overexposure white clipping. Range: 0.0..1.0
    pub overexposure: f32,
    /// Color saturation. Range: 0.0..2.0
    pub saturation: f32,
    /// Ambient background tint amount. Range: 0.0..0.1
    pub ambient: f32,
    /// Noise grain amount. Range: 0.0..0.05
    pub noise: f32,
    /// Afterglow tint color [r, g, b]. Range: 0.0..1.0 each
    pub afterglow_color: [f32; 3],
    /// Reflection mode: 0=off, 1=horizontal mirror, 2=vertical mirror, 3=quad
    pub reflection_mode: u32,
    /// Goniometer mode: Mid/Side 45 degree rotation
    pub goniometer: bool,
}

impl Default for VisualiserSettings {
    fn default() -> Self {
        Self {
            focus: 0.004,
            intensity: 1.0,
            persistence: 0.5,
            afterglow: 0.5,
            glow_amount: 0.6,
            scatter_amount: 0.4,
            color: [0.2, 1.0, 0.3],
            exposure: 1.5,
            overexposure: 0.3,
            saturation: 1.0,
            ambient: 0.02,
            noise: 0.01,
            afterglow_color: [0.2, 1.0, 0.3],
            reflection_mode: 0,
            goniometer: false,
        }
    }
}
