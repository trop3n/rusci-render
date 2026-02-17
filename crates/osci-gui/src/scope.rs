use crate::state::VisBuffer;
use nih_plug_egui::egui::{self, Vec2};
use osci_visualizer::{OsciRenderer, VisualiserSettings};
use std::sync::{Arc, Mutex};

/// Shared state for the GPU oscilloscope scope, accessed from both
/// the egui layout code and the glow paint callback.
pub struct GpuScopeState {
    pub renderer: Option<OsciRenderer>,
    pub settings: VisualiserSettings,
}

impl Default for GpuScopeState {
    fn default() -> Self {
        Self {
            renderer: None,
            settings: VisualiserSettings::default(),
        }
    }
}

/// Draw the GPU-accelerated oscilloscope scope using `egui::PaintCallback`.
pub fn draw_gpu_scope(ui: &mut egui::Ui, vis: &VisBuffer, scope_state: Arc<Mutex<GpuScopeState>>) {
    let desired_size = Vec2::splat(ui.available_width().min(300.0));
    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    // Clone sample data for the callback closure
    let x_samples = vis.x.clone();
    let y_samples = vis.y.clone();

    let cb = egui_glow::CallbackFn::new(move |info, painter| {
        let gl = painter.gl();

        let vp = info.viewport_in_pixels();
        let viewport = [
            vp.left_px,
            vp.from_bottom_px,
            vp.width_px,
            vp.height_px,
        ];

        let mut state = scope_state.lock().unwrap();

        // Lazy-initialize the renderer on first use
        if state.renderer.is_none() {
            state.renderer = Some(OsciRenderer::new(gl));
            log::info!("GPU oscilloscope renderer initialized");
        }

        // Clone settings before taking mutable borrow on renderer
        let settings = state.settings.clone();
        if let Some(renderer) = &mut state.renderer {
            renderer.render(gl, &x_samples, &y_samples, &settings, viewport);
        }
    });

    ui.painter().add(egui::PaintCallback {
        rect,
        callback: Arc::new(cb),
    });
}
