use osci_core::effect::EffectApplication;
use osci_core::envelope::Env;
use osci_core::parameter::{animate_parameter, EffectParameter};
use osci_core::Point;

use crate::renderer::ShapeRenderer;
use crate::sound::ShapeSound;

const MIN_LENGTH_INCREMENT: f64 = 0.000001;

/// Per-voice effect instance: an effect application paired with its parameters.
pub struct VoiceEffect {
    pub id: String,
    pub application: Box<dyn EffectApplication>,
    pub parameters: Vec<EffectParameter>,
    pub enabled: bool,

    // Per-parameter animation state
    animated_values: Vec<f32>,
    current_values: Vec<f32>,
}

impl VoiceEffect {
    pub fn new(
        id: impl Into<String>,
        application: Box<dyn EffectApplication>,
        parameters: Vec<EffectParameter>,
    ) -> Self {
        let n = parameters.len();
        Self {
            id: id.into(),
            application,
            parameters,
            enabled: true,
            animated_values: vec![0.0; n],
            current_values: vec![0.0; n],
        }
    }

    /// Animate all parameters for this effect over a block.
    ///
    /// After this call, `animated_values` contains the last sample's value
    /// for each parameter (suitable for per-sample effect processing).
    pub fn animate(&mut self, block_size: usize, sample_rate: f32, volume_buffer: Option<&[f32]>) {
        let mut buf = vec![0.0f32; block_size];
        for (i, param) in self.parameters.iter_mut().enumerate() {
            animate_parameter(param, &mut buf, sample_rate, &mut self.current_values[i], volume_buffer);
            self.animated_values[i] = buf[block_size - 1];
        }
    }

    /// Get the animated values for a single sample (the last animated value).
    pub fn values(&self) -> &[f32] {
        &self.animated_values
    }

    /// Create a fresh copy of this effect for another voice.
    ///
    /// Clones the effect application, parameters, and enabled state,
    /// but resets per-voice animation state to zeroes.
    pub fn clone_voice_effect(&self) -> Self {
        Self {
            id: self.id.clone(),
            application: self.application.clone_effect(),
            parameters: self.parameters.clone(),
            enabled: self.enabled,
            animated_values: vec![0.0; self.parameters.len()],
            current_values: vec![0.0; self.parameters.len()],
        }
    }
}

/// A single synthesizer voice — renders shapes to audio samples.
///
/// Mirrors the C++ `ShapeVoice`. Each voice has:
/// - A shape renderer for interpolating points along shapes
/// - An ADSR envelope
/// - A cloned effect chain
/// - MIDI state (note, velocity, pitch wheel)
pub struct ShapeVoice {
    renderer: ShapeRenderer,

    // MIDI state
    pub note: u8,
    pub velocity: f32,
    frequency: f64,
    actual_frequency: f64,
    pitch_wheel_adjustment: f64,

    // Envelope
    adsr: Env,
    time: f64,
    release_time: f64,
    end_time: f64,
    waiting_for_release: bool,

    // Voice state
    active: bool,
    sample_rate: f64,

    // Per-voice effects
    pub effects: Vec<VoiceEffect>,

    // Working buffers
    voice_x: Vec<f32>,
    voice_y: Vec<f32>,
    voice_z: Vec<f32>,
    frequency_buffer: Vec<f32>,
    volume_buffer: Vec<f32>,
}

impl ShapeVoice {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            renderer: ShapeRenderer::new(sample_rate, 60.0),
            note: 0,
            velocity: 0.0,
            frequency: 1.0,
            actual_frequency: 1.0,
            pitch_wheel_adjustment: 1.0,
            adsr: Env::adsr(0.01, 0.3, 0.5, 1.0, 1.0, -4.0),
            time: 0.0,
            release_time: 0.0,
            end_time: 99999999.0,
            waiting_for_release: false,
            active: false,
            sample_rate,
            effects: Vec::new(),
            voice_x: Vec::new(),
            voice_y: Vec::new(),
            voice_z: Vec::new(),
            frequency_buffer: Vec::new(),
            volume_buffer: Vec::new(),
        }
    }

    /// Check if this voice is currently active (playing a note).
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get the current frequency.
    pub fn frequency(&self) -> f64 {
        self.actual_frequency
    }

    /// Set the sample rate.
    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        self.renderer.set_sample_rate(sample_rate);
    }

    /// Set the ADSR envelope parameters.
    pub fn set_adsr(&mut self, adsr: Env) {
        self.adsr = adsr;
    }

    /// Start playing a MIDI note.
    pub fn start_note(
        &mut self,
        midi_note: u8,
        velocity: f32,
        sound: &mut ShapeSound,
        adsr: Env,
        midi_enabled: bool,
        default_frequency: f64,
    ) {
        self.velocity = velocity;
        self.note = midi_note;
        self.active = true;

        // Load initial frame
        let mut tries = 0;
        while sound.is_empty() && tries < 50 {
            sound.update_frame();
            tries += 1;
        }

        let frame = sound.clone_frame();
        let frame_length = osci_core::shape::total_length(&frame);
        self.renderer.set_shapes(frame);

        // Set up envelope
        self.adsr = adsr;
        self.time = 0.0;
        self.waiting_for_release = true;

        // Calculate release time and end time from ADSR
        self.release_time = 0.0;
        self.end_time = 0.0;
        let release_node = self.adsr.release_node;
        for (i, t) in self.adsr.times.iter().enumerate() {
            if (i as i32) < release_node {
                self.release_time += t;
            }
            self.end_time += t;
        }

        // Set frequency from MIDI note or default
        if midi_enabled {
            self.frequency = midi_note_to_hz(midi_note);
        } else {
            self.frequency = default_frequency;
        }

        let _ = frame_length; // frame_length is used by the renderer internally
    }

    /// Stop the note (begin release phase or immediate stop).
    pub fn stop_note(&mut self, allow_tail_off: bool) {
        self.waiting_for_release = false;
        if !allow_tail_off {
            self.note_stopped();
        }
    }

    /// Handle pitch wheel change.
    pub fn pitch_wheel_moved(&mut self, value: i32) {
        self.pitch_wheel_adjustment = 1.0 + (value as f64 - 8192.0) / 65536.0;
    }

    /// Render the next block of audio samples.
    ///
    /// Fills `output_x`, `output_y`, `output_z` with the rendered samples.
    /// The output buffers are additive — samples are mixed into existing content.
    pub fn render_next_block(
        &mut self,
        output_x: &mut [f32],
        output_y: &mut [f32],
        output_z: &mut [f32],
        num_samples: usize,
        sound: &mut ShapeSound,
        midi_enabled: bool,
        default_frequency: f64,
    ) {
        if !self.active {
            return;
        }

        // Determine frequency
        if midi_enabled {
            self.actual_frequency = self.frequency * self.pitch_wheel_adjustment;
        } else {
            self.actual_frequency = default_frequency;
        }

        // Ensure working buffers are large enough
        self.resize_buffers(num_samples);

        let frame_length = self.renderer.frame_length();

        // First pass: generate raw samples + frequency/volume buffers
        for i in 0..num_samples {
            let length_increment = if self.sample_rate > 0.0 {
                (frame_length / (self.sample_rate / self.actual_frequency)).max(MIN_LENGTH_INCREMENT)
            } else {
                MIN_LENGTH_INCREMENT
            };

            let point = self.renderer.next_vector_with_increment(length_increment);
            self.voice_x[i] = point.x;
            self.voice_y[i] = point.y;
            self.voice_z[i] = point.z;

            self.frequency_buffer[i] = self.actual_frequency as f32;

            // Envelope value for volume buffer
            let env_value = if midi_enabled {
                self.adsr.lookup(self.time as f32) as f32
            } else {
                1.0
            };
            self.volume_buffer[i] = env_value;

            // Advance time
            if self.sample_rate > 0.0 {
                self.time += 1.0 / self.sample_rate;
            }

            if self.waiting_for_release {
                self.time = self.time.min(self.release_time);
            } else if self.time >= self.end_time {
                // Zero remaining samples
                for j in (i + 1)..num_samples {
                    self.voice_x[j] = 0.0;
                    self.voice_y[j] = 0.0;
                    self.voice_z[j] = 0.0;
                    self.frequency_buffer[j] = self.actual_frequency as f32;
                    self.volume_buffer[j] = 0.0;
                }
                self.note_stopped();
                break;
            }

            // Check for frame wrap-around
            if self.renderer.frame_complete() {
                sound.update_frame();
                let new_frame = sound.clone_frame();
                self.renderer.set_shapes(new_frame);
                self.renderer.reset_frame_drawn();
            }
        }

        // Apply per-voice effects
        self.apply_effects(num_samples);

        // Apply ADSR envelope and mix into output
        for i in 0..num_samples {
            let gain = if midi_enabled {
                self.volume_buffer[i] * self.velocity
            } else {
                self.velocity.max(1.0) // Default velocity of 1 for non-MIDI
            };

            output_x[i] += self.voice_x[i] * gain;
            output_y[i] += self.voice_y[i] * gain;
            output_z[i] += self.voice_z[i] * gain;
        }
    }

    fn apply_effects(&mut self, num_samples: usize) {
        let sample_rate = self.sample_rate as f32;

        for effect in &mut self.effects {
            if !effect.enabled {
                continue;
            }

            // Animate parameters
            effect.animate(num_samples, sample_rate, Some(&self.volume_buffer));

            // Copy values to avoid borrow conflict with application
            let values: Vec<f32> = effect.animated_values.clone();
            let freq = self.actual_frequency as f32;

            // Apply effect per-sample
            for i in 0..num_samples {
                let input = Point::new(self.voice_x[i], self.voice_y[i], self.voice_z[i]);
                let external = Point::ZERO;

                let output = effect.application.apply(i, input, external, &values, sample_rate, freq);

                self.voice_x[i] = output.x;
                self.voice_y[i] = output.y;
                self.voice_z[i] = output.z;
            }
        }
    }

    fn resize_buffers(&mut self, num_samples: usize) {
        if self.voice_x.len() < num_samples {
            self.voice_x.resize(num_samples, 0.0);
            self.voice_y.resize(num_samples, 0.0);
            self.voice_z.resize(num_samples, 0.0);
            self.frequency_buffer.resize(num_samples, 0.0);
            self.volume_buffer.resize(num_samples, 0.0);
        }
    }

    fn note_stopped(&mut self) {
        self.active = false;
    }
}

/// Convert a MIDI note number to frequency in Hz.
pub fn midi_note_to_hz(note: u8) -> f64 {
    440.0 * 2.0_f64.powf((note as f64 - 69.0) / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_note_to_hz() {
        let hz = midi_note_to_hz(69); // A4
        assert!((hz - 440.0).abs() < 0.01);

        let hz = midi_note_to_hz(60); // C4
        assert!((hz - 261.626).abs() < 0.01);
    }

    #[test]
    fn test_voice_inactive_by_default() {
        let voice = ShapeVoice::new(44100.0);
        assert!(!voice.is_active());
    }

    #[test]
    fn test_voice_start_stop() {
        let mut voice = ShapeVoice::new(44100.0);
        let mut sound = ShapeSound::new(4);

        // Send a frame so the sound has something
        use osci_core::shape::Line;
        let tx = sound.sender();
        let line = Line::from_points(Point::new(-1.0, 0.0, 0.0), Point::new(1.0, 0.0, 0.0));
        tx.send(vec![Box::new(line)]).unwrap();

        let adsr = Env::adsr(0.01, 0.3, 0.5, 1.0, 1.0, -4.0);
        voice.start_note(69, 1.0, &mut sound, adsr, true, 440.0);
        assert!(voice.is_active());

        voice.stop_note(false);
        assert!(!voice.is_active());
    }
}
