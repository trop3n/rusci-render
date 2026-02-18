use nih_plug_egui::egui;

// Dracula color palette
const BACKGROUND: egui::Color32 = egui::Color32::from_rgb(40, 42, 54);
const CURRENT_LINE: egui::Color32 = egui::Color32::from_rgb(68, 71, 90);
const FOREGROUND: egui::Color32 = egui::Color32::from_rgb(248, 248, 242);
const COMMENT: egui::Color32 = egui::Color32::from_rgb(98, 114, 164);
const CYAN: egui::Color32 = egui::Color32::from_rgb(139, 233, 253);
const GREEN: egui::Color32 = egui::Color32::from_rgb(80, 250, 123);
const ORANGE: egui::Color32 = egui::Color32::from_rgb(255, 184, 108);
const PINK: egui::Color32 = egui::Color32::from_rgb(255, 121, 198);
const PURPLE: egui::Color32 = egui::Color32::from_rgb(189, 147, 249);
const RED: egui::Color32 = egui::Color32::from_rgb(255, 85, 85);
const _YELLOW: egui::Color32 = egui::Color32::from_rgb(241, 250, 140);
const EXTREME_BG: egui::Color32 = egui::Color32::from_rgb(30, 31, 41);

static INIT: std::sync::Once = std::sync::Once::new();

/// Apply the Dracula theme and Fira Sans font to the egui context.
///
/// Fonts and visuals are set once (guarded by `std::sync::Once`).
pub fn apply(ctx: &egui::Context) {
    INIT.call_once(|| {
        // -- Fonts --
        let mut fonts = egui::FontDefinitions::default();

        fonts.font_data.insert(
            "FiraSans-Regular".to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                "../../../assets/fonts/FiraSans-Regular.ttf"
            ))),
        );
        fonts.font_data.insert(
            "FiraSans-Bold".to_owned(),
            std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
                "../../../assets/fonts/FiraSans-Bold.ttf"
            ))),
        );

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "FiraSans-Regular".to_owned());

        fonts
            .families
            .entry(egui::FontFamily::Name("Bold".into()))
            .or_default()
            .insert(0, "FiraSans-Bold".to_owned());

        ctx.set_fonts(fonts);

        // -- Visuals --
        let mut visuals = egui::Visuals::dark();

        visuals.panel_fill = BACKGROUND;
        visuals.window_fill = BACKGROUND;
        visuals.faint_bg_color = CURRENT_LINE;
        visuals.extreme_bg_color = EXTREME_BG;

        visuals.selection.bg_fill = PURPLE;
        visuals.selection.stroke = egui::Stroke::new(1.0, FOREGROUND);

        visuals.hyperlink_color = CYAN;
        visuals.warn_fg_color = ORANGE;
        visuals.error_fg_color = RED;

        // Widget colors
        visuals.widgets.inactive.bg_fill = CURRENT_LINE;
        visuals.widgets.inactive.weak_bg_fill = CURRENT_LINE;
        visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, COMMENT);
        visuals.widgets.inactive.bg_stroke = egui::Stroke::new(0.0, COMMENT);

        visuals.widgets.hovered.bg_fill = COMMENT;
        visuals.widgets.hovered.weak_bg_fill = COMMENT;
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.5, FOREGROUND);
        visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, PURPLE);

        visuals.widgets.active.bg_fill = PURPLE;
        visuals.widgets.active.weak_bg_fill = PURPLE;
        visuals.widgets.active.fg_stroke = egui::Stroke::new(2.0, FOREGROUND);
        visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, PINK);

        visuals.widgets.noninteractive.bg_fill = BACKGROUND;
        visuals.widgets.noninteractive.weak_bg_fill = BACKGROUND;
        visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, FOREGROUND);
        visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(0.0, CURRENT_LINE);

        visuals.widgets.open.bg_fill = CURRENT_LINE;
        visuals.widgets.open.weak_bg_fill = CURRENT_LINE;
        visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, GREEN);
        visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, PURPLE);

        visuals.window_stroke = egui::Stroke::new(1.0, COMMENT);

        ctx.set_visuals(visuals);
    });
}
