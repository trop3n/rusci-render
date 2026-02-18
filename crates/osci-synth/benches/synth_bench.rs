use criterion::{black_box, criterion_group, criterion_main, Criterion};
use osci_core::shape::Line;
use osci_core::Point;
use osci_synth::{MidiEvent, ShapeSound, Synthesizer};

fn make_sound_with_line() -> ShapeSound {
    let mut sound = ShapeSound::new(4);
    let tx = sound.sender();
    let line = Line::from_points(Point::new(-1.0, 0.0, 0.0), Point::new(1.0, 0.0, 0.0));
    tx.send(vec![Box::new(line)]).unwrap();
    sound.update_frame();
    sound
}

fn bench_synth_render_512(c: &mut Criterion) {
    let mut synth = Synthesizer::new(16, 44100.0);
    let mut sound = make_sound_with_line();

    // Start 4 voices
    for note in [60, 64, 67, 72] {
        synth.handle_midi_event(MidiEvent::NoteOn { note, velocity: 1.0 }, &mut sound);
    }

    let num_samples = 512;
    let mut x = vec![0.0f32; num_samples];
    let mut y = vec![0.0f32; num_samples];
    let mut z = vec![0.0f32; num_samples];

    c.bench_function("synth_render_512", |b| {
        b.iter(|| {
            synth.render_next_block(
                black_box(&mut x),
                black_box(&mut y),
                black_box(&mut z),
                num_samples,
                &mut sound,
            );
        });
    });
}

fn bench_synth_render_1024(c: &mut Criterion) {
    let mut synth = Synthesizer::new(16, 44100.0);
    let mut sound = make_sound_with_line();

    for note in [60, 64, 67, 72] {
        synth.handle_midi_event(MidiEvent::NoteOn { note, velocity: 1.0 }, &mut sound);
    }

    let num_samples = 1024;
    let mut x = vec![0.0f32; num_samples];
    let mut y = vec![0.0f32; num_samples];
    let mut z = vec![0.0f32; num_samples];

    c.bench_function("synth_render_1024", |b| {
        b.iter(|| {
            synth.render_next_block(
                black_box(&mut x),
                black_box(&mut y),
                black_box(&mut z),
                num_samples,
                &mut sound,
            );
        });
    });
}

criterion_group!(benches, bench_synth_render_512, bench_synth_render_1024);
criterion_main!(benches);
