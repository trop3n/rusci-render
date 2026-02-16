use nih_plug::prelude::*;
use osci_parsers::default_shapes;
use osci_synth::{MidiEvent, ShapeSound, Synthesizer};
use std::sync::Arc;

pub struct OsciPlugin {
    params: Arc<OsciParams>,
    synth: Synthesizer,
    sound: ShapeSound,
    sample_rate: f64,
    x_buf: Vec<f32>,
    y_buf: Vec<f32>,
    z_buf: Vec<f32>,
}

#[derive(Params)]
struct OsciParams {
    #[id = "volume"]
    volume: FloatParam,
    #[id = "frequency"]
    frequency: FloatParam,
}

impl Default for OsciParams {
    fn default() -> Self {
        Self {
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
        }
    }
}

impl Default for OsciPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(OsciParams::default()),
            synth: Synthesizer::with_defaults(44100.0),
            sound: ShapeSound::new(4),
            sample_rate: 44100.0,
            x_buf: Vec::new(),
            y_buf: Vec::new(),
            z_buf: Vec::new(),
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

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate as f64;
        self.synth = Synthesizer::with_defaults(self.sample_rate);

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

        // Copy to output: X → Left, Y → Right, apply volume
        let output = buffer.as_slice();
        for i in 0..num_samples {
            output[0][i] = self.x_buf[i] * volume;
            output[1][i] = self.y_buf[i] * volume;
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
