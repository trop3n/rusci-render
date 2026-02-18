use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use osci_effects::registry::find_effect;
use osci_gui::{AudioInfo, EditorSharedState, EffectSnapshot, GpuScopeState, MenuState, OsciPluginParamRefs, UiCommand, VisBuffer};
use osci_parsers::default_shapes;
use osci_synth::{MidiEvent, ShapeSound, Synthesizer, VoiceEffect};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const VIS_BUFFER_SIZE: usize = 512;

pub struct OsciPlugin {
    params: Arc<OsciParams>,
    synth: Synthesizer,
    sound: ShapeSound,
    sample_rate: f64,
    x_buf: Vec<f32>,
    y_buf: Vec<f32>,
    z_buf: Vec<f32>,

    // Effect chain template — synced to all voices on change
    effect_template: Vec<VoiceEffect>,

    // Networking
    net_server: Option<osci_net::NetServer>,

    // UI ↔ Audio communication
    command_rx: crossbeam::channel::Receiver<UiCommand>,
    command_tx: crossbeam::channel::Sender<UiCommand>,
    effect_snapshots: Arc<Mutex<Vec<EffectSnapshot>>>,
    vis_buffer: Arc<Mutex<VisBuffer>>,
    current_project_path: Arc<Mutex<Option<PathBuf>>>,
    audio_info: Arc<Mutex<AudioInfo>>,
}

#[derive(Params)]
struct OsciParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,

    #[id = "volume"]
    volume: FloatParam,
    #[id = "frequency"]
    frequency: FloatParam,

    // ADSR envelope
    #[id = "attack"]
    attack: FloatParam,
    #[id = "decay"]
    decay: FloatParam,
    #[id = "sustain"]
    sustain: FloatParam,
    #[id = "release"]
    release: FloatParam,
}

impl Default for OsciParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(500, 700),

            volume: FloatParam::new("Volume", 1.0, FloatRange::Linear { min: 0.0, max: 3.0 }),
            frequency: FloatParam::new(
                "Frequency",
                440.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 4200.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" Hz"),

            attack: FloatParam::new(
                "Attack",
                0.01,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 2.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" s"),
            decay: FloatParam::new(
                "Decay",
                0.3,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 2.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" s"),
            sustain: FloatParam::new(
                "Sustain",
                0.5,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            ),
            release: FloatParam::new(
                "Release",
                1.0,
                FloatRange::Skewed {
                    min: 0.001,
                    max: 5.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" s"),
        }
    }
}

impl Default for OsciPlugin {
    fn default() -> Self {
        let (tx, rx) = crossbeam::channel::bounded(256);
        Self {
            params: Arc::new(OsciParams::default()),
            synth: Synthesizer::with_defaults(44100.0),
            sound: ShapeSound::new(4),
            sample_rate: 44100.0,
            x_buf: Vec::new(),
            y_buf: Vec::new(),
            z_buf: Vec::new(),
            effect_template: Vec::new(),
            net_server: None,
            command_rx: rx,
            command_tx: tx,
            effect_snapshots: Arc::new(Mutex::new(Vec::new())),
            vis_buffer: Arc::new(Mutex::new(VisBuffer::default())),
            current_project_path: Arc::new(Mutex::new(None)),
            audio_info: Arc::new(Mutex::new(AudioInfo::default())),
        }
    }
}

impl Plugin for OsciPlugin {
    const NAME: &'static str = "rusci-render";
    const VENDOR: &'static str = "rusci";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    type SysExMessage = ();
    type BackgroundTask = ();

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: None,
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let params = self.params.clone();
        let shared = EditorSharedState {
            command_tx: self.command_tx.clone(),
            effect_snapshots: self.effect_snapshots.clone(),
            vis_buffer: self.vis_buffer.clone(),
            current_project_path: self.current_project_path.clone(),
            audio_info: self.audio_info.clone(),
        };
        let scope_state = Arc::new(Mutex::new(GpuScopeState::default()));
        let menu_state = Mutex::new(MenuState::default());

        create_egui_editor(
            self.params.editor_state.clone(),
            String::new(), // selected_effect_id state
            |_, _| {},
            move |egui_ctx, setter, selected_effect_id| {
                // Lock shared state for this frame
                let snapshots = shared
                    .effect_snapshots
                    .lock()
                    .map(|s| s.clone())
                    .unwrap_or_default();
                let vis = shared
                    .vis_buffer
                    .lock()
                    .map(|v| VisBuffer {
                        x: v.x.clone(),
                        y: v.y.clone(),
                    })
                    .unwrap_or_default();

                let param_refs = OsciPluginParamRefs {
                    volume: &params.volume,
                    frequency: &params.frequency,
                    attack: &params.attack,
                    decay: &params.decay,
                    sustain: &params.sustain,
                    release: &params.release,
                };

                let scope = scope_state.clone();
                osci_gui::draw_editor(
                    egui_ctx,
                    &param_refs,
                    setter,
                    &shared,
                    &snapshots,
                    &vis,
                    selected_effect_id,
                    scope,
                    &mut menu_state.lock().unwrap(),
                );
            },
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate as f64;
        self.synth = Synthesizer::with_defaults(self.sample_rate);

        // Publish audio info for the UI
        if let Ok(mut info) = self.audio_info.lock() {
            info.sample_rate = buffer_config.sample_rate;
            info.buffer_size = buffer_config.max_buffer_size;
        }

        // Load default shapes (unit square)
        self.sound = ShapeSound::new(4);
        let tx = self.sound.sender();
        let _ = tx.send(default_shapes());
        self.sound.update_frame();

        // Allocate scratch buffers
        let max_size = buffer_config.max_buffer_size as usize;
        self.x_buf = vec![0.0; max_size];
        self.y_buf = vec![0.0; max_size];
        self.z_buf = vec![0.0; max_size];

        // Reset effect template and shared state
        self.effect_template.clear();
        if let Ok(mut snaps) = self.effect_snapshots.lock() {
            snaps.clear();
        }

        // Create fresh command channel
        let (tx, rx) = crossbeam::channel::bounded(256);
        self.command_tx = tx;
        self.command_rx = rx;

        // Start network servers
        let frame_tx = self.sound.sender();
        let sink = osci_net::FrameSink::new(frame_tx);
        self.net_server = Some(osci_net::NetServer::start(osci_net::NetConfig::default(), sink));

        true
    }

    fn reset(&mut self) {
        self.synth = Synthesizer::with_defaults(self.sample_rate);
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let num_samples = buffer.samples();

        // Read parameters
        let volume = self.params.volume.smoothed.next();
        let frequency = self.params.frequency.smoothed.next();
        self.synth.set_default_frequency(frequency as f64);

        // Build ADSR from param values
        let attack = self.params.attack.smoothed.next() as f64;
        let decay = self.params.decay.smoothed.next() as f64;
        let sustain = self.params.sustain.smoothed.next() as f64;
        let release = self.params.release.smoothed.next() as f64;
        let adsr = osci_core::Env::adsr(attack, decay, sustain, release, 1.0, -4.0);
        self.synth.set_adsr(adsr);

        // Drain UI commands
        let mut effects_changed = false;
        while let Ok(cmd) = self.command_rx.try_recv() {
            match cmd {
                UiCommand::AddEffect(id) => {
                    if let Some(entry) = find_effect(&id) {
                        let effect = VoiceEffect::new(
                            entry.id,
                            (entry.constructor)(),
                            (entry.parameters)(),
                        );
                        self.effect_template.push(effect);
                        effects_changed = true;
                    }
                }
                UiCommand::RemoveEffect(idx) => {
                    if idx < self.effect_template.len() {
                        self.effect_template.remove(idx);
                        effects_changed = true;
                    }
                }
                UiCommand::MoveEffect { from, to } => {
                    let len = self.effect_template.len();
                    if from < len && to < len && from != to {
                        let effect = self.effect_template.remove(from);
                        self.effect_template.insert(to, effect);
                        effects_changed = true;
                    }
                }
                UiCommand::SetEffectEnabled { idx, enabled } => {
                    if let Some(e) = self.effect_template.get_mut(idx) {
                        e.enabled = enabled;
                        effects_changed = true;
                    }
                }
                UiCommand::SetParamValue {
                    effect_idx,
                    param_idx,
                    value,
                } => {
                    if let Some(e) = self.effect_template.get_mut(effect_idx) {
                        if let Some(p) = e.parameters.get_mut(param_idx) {
                            p.value = value;
                            effects_changed = true;
                        }
                    }
                }
                UiCommand::SetLfo {
                    effect_idx,
                    param_idx,
                    lfo_type,
                    rate,
                    start,
                    end,
                } => {
                    if let Some(e) = self.effect_template.get_mut(effect_idx) {
                        if let Some(p) = e.parameters.get_mut(param_idx) {
                            p.lfo_type = lfo_type;
                            p.lfo_rate = rate;
                            p.lfo_start_percent = start;
                            p.lfo_end_percent = end;
                            p.lfo_enabled = !matches!(lfo_type, osci_core::LfoType::Static);
                            effects_changed = true;
                        }
                    }
                }
                UiCommand::SetSmoothing {
                    effect_idx,
                    param_idx,
                    value,
                } => {
                    if let Some(e) = self.effect_template.get_mut(effect_idx) {
                        if let Some(p) = e.parameters.get_mut(param_idx) {
                            p.smooth_value_change = value;
                            effects_changed = true;
                        }
                    }
                }
                UiCommand::SetSidechain {
                    effect_idx,
                    param_idx,
                    enabled,
                } => {
                    if let Some(e) = self.effect_template.get_mut(effect_idx) {
                        if let Some(p) = e.parameters.get_mut(param_idx) {
                            p.sidechain_enabled = enabled;
                            effects_changed = true;
                        }
                    }
                }
                UiCommand::LoadProject { effects } => {
                    self.effect_template.clear();
                    for loaded in effects {
                        if let Some(entry) = find_effect(&loaded.id) {
                            let mut effect = VoiceEffect::new(
                                entry.id,
                                (entry.constructor)(),
                                loaded.parameters,
                            );
                            effect.enabled = loaded.enabled;
                            self.effect_template.push(effect);
                        }
                    }
                    effects_changed = true;
                }
                UiCommand::ClearProject => {
                    self.effect_template.clear();
                    effects_changed = true;
                }
                UiCommand::StartRecording { .. } | UiCommand::StopRecording => {
                    // Recording commands are handled on the UI/render thread
                }
            }
        }

        // Sync effect template to all voices if anything changed
        if effects_changed {
            self.synth.set_effect_template(&self.effect_template);

            // Publish updated snapshots for the UI
            let snapshots: Vec<EffectSnapshot> = self
                .effect_template
                .iter()
                .map(|e| EffectSnapshot {
                    id: e.id.clone(),
                    name: find_effect(&e.id)
                        .map(|entry| entry.name.to_string())
                        .unwrap_or_else(|| e.id.clone()),
                    enabled: e.enabled,
                    parameters: e.parameters.clone(),
                })
                .collect();
            if let Ok(mut snaps) = self.effect_snapshots.lock() {
                *snaps = snapshots;
            }
        }

        // Drain all MIDI events (block-level processing)
        while let Some(event) = context.next_event() {
            match event {
                NoteEvent::NoteOn { note, velocity, .. } => {
                    self.synth.handle_midi_event(
                        MidiEvent::NoteOn { note, velocity },
                        &mut self.sound,
                    );
                }
                NoteEvent::NoteOff { note, velocity, .. } => {
                    self.synth.handle_midi_event(
                        MidiEvent::NoteOff { note, velocity },
                        &mut self.sound,
                    );
                }
                _ => {}
            }
        }

        // Render audio into scratch buffers
        self.synth.render_next_block(
            &mut self.x_buf[..num_samples],
            &mut self.y_buf[..num_samples],
            &mut self.z_buf[..num_samples],
            num_samples,
            &mut self.sound,
        );

        // Copy to output: X -> Left, Y -> Right, apply volume
        let output = buffer.as_slice();
        for i in 0..num_samples {
            output[0][i] = self.x_buf[i] * volume;
            output[1][i] = self.y_buf[i] * volume;
        }

        // Update vis buffer with the last VIS_BUFFER_SIZE samples
        if let Ok(mut vis) = self.vis_buffer.lock() {
            let copy_len = num_samples.min(VIS_BUFFER_SIZE);
            let src_start = num_samples.saturating_sub(VIS_BUFFER_SIZE);

            vis.x.clear();
            vis.y.clear();
            vis.x.extend_from_slice(&self.x_buf[src_start..src_start + copy_len]);
            vis.y.extend_from_slice(&self.y_buf[src_start..src_start + copy_len]);
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for OsciPlugin {
    const CLAP_ID: &'static str = "com.rusci.rusci-render";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Oscilloscope music synthesizer");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] =
        &[ClapFeature::Instrument, ClapFeature::Synthesizer, ClapFeature::Stereo];
}

impl Vst3Plugin for OsciPlugin {
    const VST3_CLASS_ID: [u8; 16] = *b"rusciRenderOsci!";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Synth];
}

nih_export_clap!(OsciPlugin);
nih_export_vst3!(OsciPlugin);
