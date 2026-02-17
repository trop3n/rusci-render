use crate::sound::ShapeSound;
use crate::voice::{ShapeVoice, VoiceEffect};
use osci_core::envelope::Env;

/// Maximum number of simultaneous voices.
const DEFAULT_MAX_VOICES: usize = 16;

/// MIDI event types used by the synthesizer.
#[derive(Debug, Clone, Copy)]
pub enum MidiEvent {
    NoteOn { note: u8, velocity: f32 },
    NoteOff { note: u8, velocity: f32 },
    PitchWheel { value: i32 },
}

/// Polyphonic synthesizer — manages multiple voices and routes MIDI events.
///
/// Mirrors the JUCE `Synthesiser` class behavior. Voices are allocated on
/// note-on and released on note-off. When all voices are in use, the oldest
/// voice is stolen.
pub struct Synthesizer {
    voices: Vec<ShapeVoice>,
    sample_rate: f64,
    adsr: Env,
    midi_enabled: bool,
    default_frequency: f64,
}

impl Synthesizer {
    /// Create a new synthesizer with the given number of voices.
    pub fn new(num_voices: usize, sample_rate: f64) -> Self {
        let mut voices = Vec::with_capacity(num_voices);
        for _ in 0..num_voices {
            voices.push(ShapeVoice::new(sample_rate));
        }

        Self {
            voices,
            sample_rate,
            adsr: Env::adsr(0.01, 0.3, 0.5, 1.0, 1.0, -4.0),
            midi_enabled: true,
            default_frequency: 440.0,
        }
    }

    /// Create a synthesizer with the default number of voices.
    pub fn with_defaults(sample_rate: f64) -> Self {
        Self::new(DEFAULT_MAX_VOICES, sample_rate)
    }

    /// Set the sample rate for all voices.
    pub fn set_sample_rate(&mut self, sample_rate: f64) {
        self.sample_rate = sample_rate;
        for voice in &mut self.voices {
            voice.set_sample_rate(sample_rate);
        }
    }

    /// Set the ADSR envelope that new notes will use.
    pub fn set_adsr(&mut self, adsr: Env) {
        self.adsr = adsr;
    }

    /// Enable or disable MIDI mode.
    ///
    /// When disabled, the synthesizer uses the default frequency and ignores
    /// MIDI note numbers.
    pub fn set_midi_enabled(&mut self, enabled: bool) {
        self.midi_enabled = enabled;
    }

    /// Set the default frequency used when MIDI is disabled.
    pub fn set_default_frequency(&mut self, frequency: f64) {
        self.default_frequency = frequency;
    }

    /// Get a mutable reference to a voice by index.
    pub fn voice_mut(&mut self, index: usize) -> Option<&mut ShapeVoice> {
        self.voices.get_mut(index)
    }

    /// Get the number of currently active voices.
    pub fn active_voice_count(&self) -> usize {
        self.voices.iter().filter(|v| v.is_active()).count()
    }

    /// Get the total number of voice slots.
    pub fn num_voices(&self) -> usize {
        self.voices.len()
    }

    /// Sync an effect template to all voices.
    ///
    /// Each voice gets a fresh clone of every effect in the template,
    /// with per-voice animation state reset to zeroes.
    pub fn set_effect_template(&mut self, template: &[VoiceEffect]) {
        for voice in &mut self.voices {
            voice.effects = template.iter().map(|e| e.clone_voice_effect()).collect();
        }
    }

    /// Process a MIDI event.
    pub fn handle_midi_event(&mut self, event: MidiEvent, sound: &mut ShapeSound) {
        match event {
            MidiEvent::NoteOn { note, velocity } => {
                self.note_on(note, velocity, sound);
            }
            MidiEvent::NoteOff { note, velocity: _ } => {
                self.note_off(note);
            }
            MidiEvent::PitchWheel { value } => {
                for voice in &mut self.voices {
                    if voice.is_active() {
                        voice.pitch_wheel_moved(value);
                    }
                }
            }
        }
    }

    /// Render the next block of audio from all active voices.
    ///
    /// The output is written to `output_x`, `output_y`, `output_z`.
    /// These buffers are cleared before rendering, then all active voices
    /// are mixed additively.
    pub fn render_next_block(
        &mut self,
        output_x: &mut [f32],
        output_y: &mut [f32],
        output_z: &mut [f32],
        num_samples: usize,
        sound: &mut ShapeSound,
    ) {
        // Clear output buffers
        for i in 0..num_samples {
            output_x[i] = 0.0;
            output_y[i] = 0.0;
            output_z[i] = 0.0;
        }

        // Render each active voice into the output
        for voice in &mut self.voices {
            if voice.is_active() {
                voice.render_next_block(
                    output_x,
                    output_y,
                    output_z,
                    num_samples,
                    sound,
                    self.midi_enabled,
                    self.default_frequency,
                );
            }
        }
    }

    fn note_on(&mut self, note: u8, velocity: f32, sound: &mut ShapeSound) {
        // Find a free voice, or steal the oldest
        let voice_idx = self.find_free_voice().unwrap_or_else(|| self.steal_voice());

        let voice = &mut self.voices[voice_idx];
        voice.start_note(
            note,
            velocity,
            sound,
            self.adsr.clone(),
            self.midi_enabled,
            self.default_frequency,
        );
    }

    fn note_off(&mut self, note: u8) {
        for voice in &mut self.voices {
            if voice.is_active() && voice.note == note {
                voice.stop_note(true);
            }
        }
    }

    fn find_free_voice(&self) -> Option<usize> {
        self.voices.iter().position(|v| !v.is_active())
    }

    fn steal_voice(&mut self) -> usize {
        // Simple voice stealing: stop the first voice
        // A more sophisticated approach would steal the quietest or oldest
        if let Some(idx) = self.voices.iter().position(|v| v.is_active()) {
            self.voices[idx].stop_note(false);
            idx
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use osci_core::shape::Line;
    use osci_core::Point;

    fn make_sound_with_line() -> ShapeSound {
        let mut sound = ShapeSound::new(4);
        let tx = sound.sender();
        let line = Line::from_points(Point::new(-1.0, 0.0, 0.0), Point::new(1.0, 0.0, 0.0));
        tx.send(vec![Box::new(line)]).unwrap();
        sound.update_frame();
        sound
    }

    #[test]
    fn test_synth_creation() {
        let synth = Synthesizer::with_defaults(44100.0);
        assert_eq!(synth.active_voice_count(), 0);
    }

    #[test]
    fn test_note_on_off() {
        let mut synth = Synthesizer::new(4, 44100.0);
        let mut sound = make_sound_with_line();

        synth.handle_midi_event(
            MidiEvent::NoteOn { note: 69, velocity: 1.0 },
            &mut sound,
        );
        assert_eq!(synth.active_voice_count(), 1);

        synth.handle_midi_event(
            MidiEvent::NoteOff { note: 69, velocity: 0.0 },
            &mut sound,
        );
        // Voice may still be active (in release phase) depending on tail-off
    }

    #[test]
    fn test_render_block() {
        let mut synth = Synthesizer::new(4, 44100.0);
        let mut sound = make_sound_with_line();

        synth.handle_midi_event(
            MidiEvent::NoteOn { note: 69, velocity: 1.0 },
            &mut sound,
        );

        let num_samples = 128;
        let mut x = vec![0.0f32; num_samples];
        let mut y = vec![0.0f32; num_samples];
        let mut z = vec![0.0f32; num_samples];

        synth.render_next_block(&mut x, &mut y, &mut z, num_samples, &mut sound);

        // Should have non-zero output
        let has_output = x.iter().any(|v| v.abs() > 0.001)
            || y.iter().any(|v| v.abs() > 0.001);
        assert!(has_output);
    }

    #[test]
    fn test_voice_stealing() {
        let mut synth = Synthesizer::new(2, 44100.0);
        let mut sound = make_sound_with_line();

        // Fill all voices
        synth.handle_midi_event(MidiEvent::NoteOn { note: 60, velocity: 1.0 }, &mut sound);
        synth.handle_midi_event(MidiEvent::NoteOn { note: 64, velocity: 1.0 }, &mut sound);
        assert_eq!(synth.active_voice_count(), 2);

        // Third note should steal a voice
        synth.handle_midi_event(MidiEvent::NoteOn { note: 67, velocity: 1.0 }, &mut sound);
        assert_eq!(synth.active_voice_count(), 2);
    }
}
