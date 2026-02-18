use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use osci_gui::{GpuScopeState, VisBuffer};
use osci_visualizer::VisualiserSettings;
use std::sync::{Arc, Mutex};

const VIS_BUFFER_SIZE: usize = 512;

pub struct RusciPlugin {
    params: Arc<RusciParams>,
    vis_buffer: Arc<Mutex<VisBuffer>>,
}

#[derive(Params)]
struct RusciParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,
}

impl Default for RusciParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(520, 850),
        }
    }
}

impl Default for RusciPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(RusciParams::default()),
            vis_buffer: Arc::new(Mutex::new(VisBuffer::default())),
        }
    }
}

impl Plugin for RusciPlugin {
    const NAME: &'static str = "Rusci";
    const VENDOR: &'static str = "rusci";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    type SysExMessage = ();
    type BackgroundTask = ();

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let vis_buffer = self.vis_buffer.clone();
        let scope_state = Arc::new(Mutex::new(GpuScopeState::default()));

        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_, _| {},
            move |egui_ctx, _setter, _state| {
                osci_gui::theme::apply(egui_ctx);

                // Snapshot the vis buffer for this frame
                let vis = vis_buffer
                    .lock()
                    .map(|v| VisBuffer {
                        x: v.x.clone(),
                        y: v.y.clone(),
                    })
                    .unwrap_or_default();

                let scope = scope_state.clone();

                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        // XY Scope (GPU-rendered)
                        ui.heading("XY Scope");
                        ui.separator();
                        osci_gui::scope::draw_gpu_scope(ui, &vis, scope.clone());

                        ui.add_space(12.0);

                        // Visualizer settings
                        if let Ok(mut state) = scope.lock() {
                            draw_visualizer_settings(ui, &mut state.settings);
                        }
                    });
                });
            },
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let num_samples = buffer.samples();

        // Audio passthrough: input is already in the buffer, nothing to do.
        // nih-plug passes input data through to output by default for matching layouts.

        // Update vis buffer with the last VIS_BUFFER_SIZE samples
        if let Ok(mut vis) = self.vis_buffer.lock() {
            let output = buffer.as_slice();
            let copy_len = num_samples.min(VIS_BUFFER_SIZE);
            let src_start = num_samples.saturating_sub(VIS_BUFFER_SIZE);

            vis.x.clear();
            vis.y.clear();
            vis.x.extend_from_slice(&output[0][src_start..src_start + copy_len]);
            vis.y.extend_from_slice(&output[1][src_start..src_start + copy_len]);
        }

        ProcessStatus::Normal
    }
}

fn draw_visualizer_settings(ui: &mut egui::Ui, s: &mut VisualiserSettings) {
    // -- Beam --
    ui.heading("Beam");
    ui.separator();
    ui.add(egui::Slider::new(&mut s.focus, 0.001..=0.02).text("Focus"));
    ui.add(egui::Slider::new(&mut s.intensity, 0.1..=5.0).text("Intensity"));
    ui.horizontal(|ui| {
        ui.label("Color");
        ui.add(egui::Slider::new(&mut s.color[0], 0.0..=1.0).text("R"));
        ui.add(egui::Slider::new(&mut s.color[1], 0.0..=1.0).text("G"));
        ui.add(egui::Slider::new(&mut s.color[2], 0.0..=1.0).text("B"));
    });

    ui.add_space(8.0);

    // -- Glow --
    ui.heading("Glow");
    ui.separator();
    ui.add(egui::Slider::new(&mut s.glow_amount, 0.0..=2.0).text("Glow"));
    ui.add(egui::Slider::new(&mut s.scatter_amount, 0.0..=2.0).text("Scatter"));
    ui.add(egui::Slider::new(&mut s.persistence, 0.0..=1.0).text("Persistence"));
    ui.add(egui::Slider::new(&mut s.afterglow, 0.0..=1.0).text("Afterglow"));
    ui.horizontal(|ui| {
        ui.label("Afterglow Color");
        ui.add(egui::Slider::new(&mut s.afterglow_color[0], 0.0..=1.0).text("R"));
        ui.add(egui::Slider::new(&mut s.afterglow_color[1], 0.0..=1.0).text("G"));
        ui.add(egui::Slider::new(&mut s.afterglow_color[2], 0.0..=1.0).text("B"));
    });

    ui.add_space(8.0);

    // -- Tone --
    ui.heading("Tone");
    ui.separator();
    ui.add(egui::Slider::new(&mut s.exposure, 0.5..=5.0).text("Exposure"));
    ui.add(egui::Slider::new(&mut s.overexposure, 0.0..=1.0).text("Overexposure"));
    ui.add(egui::Slider::new(&mut s.saturation, 0.0..=2.0).text("Saturation"));
    ui.add(egui::Slider::new(&mut s.ambient, 0.0..=0.1).text("Ambient"));
    ui.add(egui::Slider::new(&mut s.noise, 0.0..=0.05).text("Noise"));

    ui.add_space(8.0);

    // -- Mode --
    ui.heading("Mode");
    ui.separator();

    let mode_label = match s.reflection_mode {
        0 => "Off",
        1 => "Horizontal",
        2 => "Vertical",
        3 => "Quad",
        _ => "Off",
    };
    egui::ComboBox::from_label("Reflection")
        .selected_text(mode_label)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut s.reflection_mode, 0, "Off");
            ui.selectable_value(&mut s.reflection_mode, 1, "Horizontal");
            ui.selectable_value(&mut s.reflection_mode, 2, "Vertical");
            ui.selectable_value(&mut s.reflection_mode, 3, "Quad");
        });

    ui.checkbox(&mut s.goniometer, "Goniometer (Mid/Side rotation)");
}

impl ClapPlugin for RusciPlugin {
    const CLAP_ID: &'static str = "com.rusci.rusci";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Audio-input oscilloscope visualizer");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Analyzer,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for RusciPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"rusciOsciScope!_";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[
        Vst3SubCategory::Fx,
        Vst3SubCategory::Analyzer,
    ];
}

nih_export_clap!(RusciPlugin);
nih_export_vst3!(RusciPlugin);
