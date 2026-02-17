use crate::state::VisBuffer;
use nih_plug_egui::egui::{self, Color32, Pos2, Stroke, StrokeKind, Vec2};

/// Draw an XY Lissajous-style oscilloscope in a square region.
///
/// Reads from a `VisBuffer` and draws green lines connecting consecutive
/// (x, y) sample points on a black background.
pub fn draw_scope(ui: &mut egui::Ui, vis: &VisBuffer) {
    let desired_size = Vec2::splat(ui.available_width().min(300.0));
    let (response, painter) = ui.allocate_painter(desired_size, egui::Sense::hover());
    let rect = response.rect;

    // Black background
    painter.rect_filled(rect, 0.0, Color32::BLACK);

    // Draw crosshair guides (dim)
    let center = rect.center();
    let guide_stroke = Stroke::new(0.5, Color32::from_gray(40));
    painter.line_segment(
        [Pos2::new(rect.left(), center.y), Pos2::new(rect.right(), center.y)],
        guide_stroke,
    );
    painter.line_segment(
        [Pos2::new(center.x, rect.top()), Pos2::new(center.x, rect.bottom())],
        guide_stroke,
    );

    let n = vis.x.len().min(vis.y.len());
    if n < 2 {
        return;
    }

    let stroke = Stroke::new(1.5, Color32::GREEN);

    // Map sample coordinates [-1, 1] to the drawing rect
    let map = |x: f32, y: f32| -> Pos2 {
        Pos2::new(
            remap(x, -1.0, 1.0, rect.left(), rect.right()),
            remap(-y, -1.0, 1.0, rect.top(), rect.bottom()), // flip Y for screen coords
        )
    };

    // Draw lines between consecutive points
    let mut prev = map(vis.x[0], vis.y[0]);
    for i in 1..n {
        let cur = map(vis.x[i], vis.y[i]);
        // Skip degenerate segments
        if prev.distance(cur) < rect.width() * 2.0 {
            painter.line_segment([prev, cur], stroke);
        }
        prev = cur;
    }

    // Border
    painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::from_gray(60)), StrokeKind::Outside);
}

/// Linear remap from [in_min, in_max] to [out_min, out_max], clamped.
fn remap(value: f32, in_min: f32, in_max: f32, out_min: f32, out_max: f32) -> f32 {
    let t = ((value - in_min) / (in_max - in_min)).clamp(0.0, 1.0);
    out_min + t * (out_max - out_min)
}
