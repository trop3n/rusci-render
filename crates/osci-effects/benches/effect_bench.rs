use criterion::{black_box, criterion_group, criterion_main, Criterion};
use osci_core::{EffectApplication, Point};
use osci_effects::rotate::Rotate;
use osci_effects::smooth::SmoothEffect;
use osci_effects::scale::ScaleEffect;

fn bench_rotate_512(c: &mut Criterion) {
    let mut effect = Rotate::new();
    let values = [0.25_f32, 0.5, 0.0];
    let input = Point::new(0.5, 0.3, 0.1);
    let ext = Point::ZERO;

    c.bench_function("rotate_512_samples", |b| {
        b.iter(|| {
            for i in 0..512 {
                black_box(effect.apply(i, input, ext, &values, 44100.0, 440.0));
            }
        });
    });
}

fn bench_smooth_512(c: &mut Criterion) {
    let mut effect = SmoothEffect::new();
    let values = [0.5_f32];
    let ext = Point::ZERO;

    c.bench_function("smooth_512_samples", |b| {
        b.iter(|| {
            for i in 0..512 {
                let input = Point::new(
                    (i as f32 * 0.1).sin(),
                    (i as f32 * 0.1).cos(),
                    0.0,
                );
                black_box(effect.apply(i, input, ext, &values, 44100.0, 440.0));
            }
        });
    });
}

fn bench_scale_512(c: &mut Criterion) {
    let mut effect = ScaleEffect::new();
    let values = [1.5_f32, 0.8, 1.0];
    let input = Point::new(0.5, 0.3, 0.1);
    let ext = Point::ZERO;

    c.bench_function("scale_512_samples", |b| {
        b.iter(|| {
            for i in 0..512 {
                black_box(effect.apply(i, input, ext, &values, 44100.0, 440.0));
            }
        });
    });
}

criterion_group!(benches, bench_rotate_512, bench_smooth_512, bench_scale_512);
criterion_main!(benches);
