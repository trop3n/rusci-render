use osci_core::envelope::Env;
use osci_core::shape::Line;
use osci_core::Point;
use osci_synth::voice::VoiceEffect;
use osci_synth::{
    FrameProducer, MidiEvent, ShapeSound, StaticFrameSource, Synthesizer,
};

// ── Helpers ──────────────────────────────────────────────────────

const SAMPLE_RATE: f64 = 44100.0;
const BLOCK_SIZE: usize = 512;

/// Create a ShapeSound pre-loaded with a horizontal line from -1 to +1.
fn make_sound_with_line() -> ShapeSound {
    let mut sound = ShapeSound::new(4);
    let tx = sound.sender();
    let line = Line::from_points(
        Point::new(-1.0, 0.0, 0.0),
        Point::new(1.0, 0.0, 0.0),
    );
    tx.send(vec![Box::new(line)]).unwrap();
    sound.update_frame();
    sound
}

/// Create a ShapeSound with a unit square (4 lines).
fn make_sound_with_square() -> ShapeSound {
    let mut sound = ShapeSound::new(4);
    let tx = sound.sender();
    let frame: Vec<Box<dyn osci_core::shape::Shape>> = vec![
        Box::new(Line::new_2d(-0.5, -0.5, 0.5, -0.5)),
        Box::new(Line::new_2d(0.5, -0.5, 0.5, 0.5)),
        Box::new(Line::new_2d(0.5, 0.5, -0.5, 0.5)),
        Box::new(Line::new_2d(-0.5, 0.5, -0.5, -0.5)),
    ];
    tx.send(frame).unwrap();
    sound.update_frame();
    sound
}

/// Render one block and return (x, y, z) buffers.
fn render_block(
    synth: &mut Synthesizer,
    sound: &mut ShapeSound,
    num_samples: usize,
) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut x = vec![0.0f32; num_samples];
    let mut y = vec![0.0f32; num_samples];
    let mut z = vec![0.0f32; num_samples];
    synth.render_next_block(&mut x, &mut y, &mut z, num_samples, sound);
    (x, y, z)
}

fn has_nonzero(buf: &[f32], threshold: f32) -> bool {
    buf.iter().any(|v| v.abs() > threshold)
}

fn all_finite(buf: &[f32]) -> bool {
    buf.iter().all(|v| v.is_finite())
}

// ── 1. Basic rendering ──────────────────────────────────────────

#[test]
fn basic_render_produces_output() {
    let mut synth = Synthesizer::new(4, SAMPLE_RATE);
    let mut sound = make_sound_with_line();

    synth.handle_midi_event(
        MidiEvent::NoteOn { note: 69, velocity: 1.0 },
        &mut sound,
    );

    let (x, _y, _z) = render_block(&mut synth, &mut sound, BLOCK_SIZE);

    assert!(
        has_nonzero(&x, 0.001),
        "rendering with active note should produce non-zero X output"
    );
}

#[test]
fn no_notes_produces_silence() {
    let mut synth = Synthesizer::new(4, SAMPLE_RATE);
    let mut sound = make_sound_with_line();

    let (x, y, _z) = render_block(&mut synth, &mut sound, BLOCK_SIZE);

    assert!(
        !has_nonzero(&x, 0.0001) && !has_nonzero(&y, 0.0001),
        "no active notes should produce silence"
    );
}

// ── 2. Output bounds ─────────────────────────────────────────────

#[test]
fn output_is_finite_and_bounded() {
    let mut synth = Synthesizer::new(4, SAMPLE_RATE);
    let mut sound = make_sound_with_square();

    synth.handle_midi_event(
        MidiEvent::NoteOn { note: 60, velocity: 1.0 },
        &mut sound,
    );

    // Render many blocks
    for _ in 0..20 {
        let (x, y, z) = render_block(&mut synth, &mut sound, BLOCK_SIZE);
        assert!(all_finite(&x), "X output contains non-finite values");
        assert!(all_finite(&y), "Y output contains non-finite values");
        assert!(all_finite(&z), "Z output contains non-finite values");

        // All samples should be within a reasonable range
        for i in 0..BLOCK_SIZE {
            assert!(
                x[i].abs() < 10.0 && y[i].abs() < 10.0 && z[i].abs() < 10.0,
                "sample {} out of bounds: ({}, {}, {})",
                i, x[i], y[i], z[i]
            );
        }
    }
}

// ── 3. Multi-voice mixing ────────────────────────────────────────

#[test]
fn multi_voice_produces_combined_output() {
    let mut synth = Synthesizer::new(8, SAMPLE_RATE);
    let mut sound = make_sound_with_line();

    // Single note
    synth.handle_midi_event(
        MidiEvent::NoteOn { note: 60, velocity: 1.0 },
        &mut sound,
    );
    let (x1, _, _) = render_block(&mut synth, &mut sound, BLOCK_SIZE);
    let energy1: f32 = x1.iter().map(|v| v * v).sum();

    // Reset
    synth.handle_midi_event(
        MidiEvent::NoteOff { note: 60, velocity: 0.0 },
        &mut sound,
    );

    // Re-create synth to get fresh voices
    let mut synth = Synthesizer::new(8, SAMPLE_RATE);
    let mut sound = make_sound_with_line();

    // Two simultaneous notes
    synth.handle_midi_event(
        MidiEvent::NoteOn { note: 60, velocity: 1.0 },
        &mut sound,
    );
    synth.handle_midi_event(
        MidiEvent::NoteOn { note: 67, velocity: 1.0 },
        &mut sound,
    );

    assert_eq!(synth.active_voice_count(), 2);

    let (x2, _, _) = render_block(&mut synth, &mut sound, BLOCK_SIZE);
    let energy2: f32 = x2.iter().map(|v| v * v).sum();

    // Two voices should produce more energy than one
    assert!(
        energy2 > energy1 * 0.5,
        "two voices should produce at least comparable energy: single={energy1}, double={energy2}"
    );
}

// ── 4. Voice lifecycle ───────────────────────────────────────────

#[test]
fn voice_lifecycle_note_on_off_release() {
    // Use a very short envelope so release finishes quickly
    let mut synth = Synthesizer::new(4, SAMPLE_RATE);
    let short_adsr = Env::adsr(0.001, 0.001, 1.0, 0.01, 1.0, -4.0);
    synth.set_adsr(short_adsr);

    let mut sound = make_sound_with_line();

    synth.handle_midi_event(
        MidiEvent::NoteOn { note: 69, velocity: 1.0 },
        &mut sound,
    );
    assert_eq!(synth.active_voice_count(), 1);

    // Render a block during sustain
    let (x, _, _) = render_block(&mut synth, &mut sound, BLOCK_SIZE);
    assert!(has_nonzero(&x, 0.001), "voice should produce output while sustained");

    // Note off — voice enters release
    synth.handle_midi_event(
        MidiEvent::NoteOff { note: 69, velocity: 0.0 },
        &mut sound,
    );

    // Render enough blocks for ADSR release to finish (0.01s release ~ 441 samples)
    // Render several blocks to be safe
    for _ in 0..20 {
        render_block(&mut synth, &mut sound, BLOCK_SIZE);
    }

    // After release, voice should become inactive
    assert_eq!(
        synth.active_voice_count(),
        0,
        "voice should become inactive after release"
    );

    // Output should be zero once voice is inactive
    let (x, y, _) = render_block(&mut synth, &mut sound, BLOCK_SIZE);
    assert!(
        !has_nonzero(&x, 0.0001) && !has_nonzero(&y, 0.0001),
        "output should be silent after voice release"
    );
}

// ── 5. Effects in voice chain ────────────────────────────────────

#[test]
fn voice_effect_translate_modifies_output() {
    // Render without effect
    let mut synth_plain = Synthesizer::new(4, SAMPLE_RATE);
    synth_plain.set_midi_enabled(false);
    synth_plain.set_default_frequency(60.0);
    let mut sound_plain = make_sound_with_line();

    synth_plain.handle_midi_event(
        MidiEvent::NoteOn { note: 69, velocity: 1.0 },
        &mut sound_plain,
    );
    let (x_plain, _, _) = render_block(&mut synth_plain, &mut sound_plain, BLOCK_SIZE);

    // Render with translate effect
    let mut synth_fx = Synthesizer::new(4, SAMPLE_RATE);
    synth_fx.set_midi_enabled(false);
    synth_fx.set_default_frequency(60.0);
    let mut sound_fx = make_sound_with_line();

    synth_fx.handle_midi_event(
        MidiEvent::NoteOn { note: 69, velocity: 1.0 },
        &mut sound_fx,
    );

    // Add translate effect to voice 0
    let translate_entry = osci_effects::registry::find_effect("translate").unwrap();
    let mut params = (translate_entry.parameters)();
    // Set translateX to 0.5
    params[0].value = 0.5;
    params[0].default_value = 0.5;
    let voice_effect = VoiceEffect::new(
        "translate",
        (translate_entry.constructor)(),
        params,
    );
    if let Some(voice) = synth_fx.voice_mut(0) {
        voice.effects.push(voice_effect);
    }

    let (x_fx, _, _) = render_block(&mut synth_fx, &mut sound_fx, BLOCK_SIZE);

    // The effected output should differ from the plain output
    let diff: f32 = x_plain.iter().zip(x_fx.iter()).map(|(a, b)| (a - b).abs()).sum();
    assert!(
        diff > 0.1,
        "translate effect should change X output, total diff={diff}"
    );
}

#[test]
fn voice_effect_volume_attenuates() {
    let mut synth = Synthesizer::new(4, SAMPLE_RATE);
    synth.set_midi_enabled(false);
    synth.set_default_frequency(60.0);
    let mut sound = make_sound_with_line();

    synth.handle_midi_event(
        MidiEvent::NoteOn { note: 69, velocity: 1.0 },
        &mut sound,
    );

    // Add volume effect at 0.0 — should silence output
    let volume_entry = osci_effects::registry::find_effect("volume").unwrap();
    let mut params = (volume_entry.parameters)();
    params[0].value = 0.0;
    params[0].default_value = 0.0;
    let voice_effect = VoiceEffect::new(
        "volume",
        (volume_entry.constructor)(),
        params,
    );
    if let Some(voice) = synth.voice_mut(0) {
        voice.effects.push(voice_effect);
    }

    let (x, y, _) = render_block(&mut synth, &mut sound, BLOCK_SIZE);

    // Volume=0 should zero the spatial coordinates
    assert!(
        !has_nonzero(&x, 0.001) && !has_nonzero(&y, 0.001),
        "volume=0 effect should silence X and Y"
    );
}

// ── 6. Frame producer pipeline ───────────────────────────────────

#[test]
fn frame_producer_feeds_synthesizer() {
    let line = Line::from_points(
        Point::new(-1.0, -1.0, 0.0),
        Point::new(1.0, 1.0, 0.0),
    );
    let source = StaticFrameSource::new(vec![Box::new(line)]);

    let mut sound = ShapeSound::new(4);
    let frame_tx = sound.sender();

    let _producer = FrameProducer::start(source, frame_tx);

    // Give the producer time to send a frame
    std::thread::sleep(std::time::Duration::from_millis(50));
    sound.update_frame();

    let mut synth = Synthesizer::new(4, SAMPLE_RATE);
    synth.handle_midi_event(
        MidiEvent::NoteOn { note: 69, velocity: 1.0 },
        &mut sound,
    );

    // Render several blocks, draining the channel so the producer doesn't block
    let mut any_nonzero = false;
    for _ in 0..5 {
        sound.update_frame(); // drain channel to prevent producer from blocking
        let (x, y, _) = render_block(&mut synth, &mut sound, BLOCK_SIZE);
        if has_nonzero(&x, 0.001) || has_nonzero(&y, 0.001) {
            any_nonzero = true;
        }
    }

    assert!(any_nonzero, "frame producer pipeline should produce non-zero output");

    // Drop sound first to disconnect the channel, allowing the producer to exit
    drop(sound);
    // _producer is dropped here, which calls stop() — the thread will exit
    // because the channel is disconnected
}

// ── 7. SVG-to-audio end-to-end ───────────────────────────────────

#[test]
fn svg_to_audio_e2e() {
    use osci_parsers::{parse_file, ParseResult};

    let svg_data = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
      <rect x="10" y="10" width="80" height="80" fill="none" stroke="black"/>
    </svg>"#;

    let shapes = match parse_file(svg_data, "svg").expect("SVG should parse") {
        ParseResult::Shapes(s) => s,
        _ => panic!("expected shapes from SVG"),
    };
    assert!(!shapes.is_empty(), "SVG should produce shapes");

    // Feed shapes into synth pipeline
    let mut sound = ShapeSound::new(4);
    let tx = sound.sender();
    tx.send(shapes).unwrap();
    sound.update_frame();

    let mut synth = Synthesizer::new(4, SAMPLE_RATE);
    synth.handle_midi_event(
        MidiEvent::NoteOn { note: 69, velocity: 1.0 },
        &mut sound,
    );

    let (x, y, z) = render_block(&mut synth, &mut sound, BLOCK_SIZE);

    assert!(all_finite(&x), "SVG E2E X output should be finite");
    assert!(all_finite(&y), "SVG E2E Y output should be finite");
    assert!(all_finite(&z), "SVG E2E Z output should be finite");
    assert!(
        has_nonzero(&x, 0.001) || has_nonzero(&y, 0.001),
        "SVG E2E should produce non-zero output"
    );
}

// ── 8. Frequency correctness ─────────────────────────────────────

#[test]
fn frequency_approximation_from_zero_crossings() {
    let target_freq = 100.0; // Hz — low enough for clear zero crossings
    let mut synth = Synthesizer::new(4, SAMPLE_RATE);
    synth.set_midi_enabled(false);
    synth.set_default_frequency(target_freq);

    let mut sound = make_sound_with_line();

    synth.handle_midi_event(
        MidiEvent::NoteOn { note: 69, velocity: 1.0 },
        &mut sound,
    );

    // Render a large block for analysis
    let num_samples = 44100; // 1 second
    let (x, _, _) = render_block(&mut synth, &mut sound, num_samples);

    // Count zero crossings in x channel
    let mut crossings = 0u32;
    for i in 1..num_samples {
        if (x[i - 1] >= 0.0 && x[i] < 0.0) || (x[i - 1] < 0.0 && x[i] >= 0.0) {
            crossings += 1;
        }
    }

    // Each complete cycle has 2 zero crossings, so estimated_freq = crossings / 2
    let estimated_freq = crossings as f64 / 2.0;

    // Allow generous tolerance since shape traversal isn't a pure sine
    assert!(
        estimated_freq > target_freq * 0.3 && estimated_freq < target_freq * 3.0,
        "estimated frequency {estimated_freq} Hz is too far from target {target_freq} Hz (crossings={crossings})"
    );
}
