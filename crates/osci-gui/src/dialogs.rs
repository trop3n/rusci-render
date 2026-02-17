use crate::state::AudioInfo;
use nih_plug_egui::egui;

/// Draw the About dialog window.
pub fn draw_about_dialog(ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("About rusci-render")
        .open(open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("rusci-render");
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.add_space(8.0);
                ui.label("Oscilloscope music synthesizer");
                ui.label("A real-time XY oscilloscope synthesizer with");
                ui.label("audio effects, visualization, and MIDI support.");
                ui.add_space(8.0);
                ui.label("License: GPL-3.0");
            });
        });
}

/// Draw the Audio Device Info dialog window.
pub fn draw_audio_info_dialog(ctx: &egui::Context, open: &mut bool, info: &AudioInfo) {
    egui::Window::new("Audio Device Info")
        .open(open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Grid::new("audio_info_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Sample Rate:");
                    ui.label(format!("{} Hz", info.sample_rate));
                    ui.end_row();

                    ui.label("Buffer Size:");
                    ui.label(format!("{} samples", info.buffer_size));
                    ui.end_row();

                    ui.label("Latency:");
                    if info.sample_rate > 0.0 {
                        let latency_ms =
                            (info.buffer_size as f64 / info.sample_rate as f64) * 1000.0;
                        ui.label(format!("{:.1} ms", latency_ms));
                    } else {
                        ui.label("N/A");
                    }
                    ui.end_row();
                });
            ui.add_space(8.0);
            ui.separator();
            ui.label("Standalone: use --device to select audio device.");
            ui.label("Plugin: audio device is managed by the host.");
        });
}

/// Draw the Keyboard Shortcuts dialog window.
pub fn draw_shortcuts_dialog(ctx: &egui::Context, open: &mut bool) {
    egui::Window::new("Keyboard Shortcuts")
        .open(open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            egui::Grid::new("shortcuts_grid")
                .num_columns(2)
                .spacing([40.0, 4.0])
                .show(ui, |ui| {
                    let shortcuts = [
                        ("Ctrl+N", "New Project"),
                        ("Ctrl+O", "Open Project"),
                        ("Ctrl+S", "Save Project"),
                        ("Ctrl+Shift+S", "Save Project As"),
                    ];
                    for (key, desc) in shortcuts {
                        ui.label(
                            egui::RichText::new(key).monospace(),
                        );
                        ui.label(desc);
                        ui.end_row();
                    }
                });
        });
}
