use criterion::{black_box, criterion_group, criterion_main, Criterion};
use osci_core::shape::{CircleArc, CubicBezierCurve, Line, Shape, total_length};
use osci_core::Point;

fn bench_line_sample_1000(c: &mut Criterion) {
    let line = Line::new_2d(-1.0, -1.0, 1.0, 1.0);
    c.bench_function("line_sample_1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                black_box(line.next_vector(i as f32 / 1000.0));
            }
        });
    });
}

fn bench_cubic_bezier_sample_1000(c: &mut Criterion) {
    let curve = CubicBezierCurve::new(0.0, 0.0, 0.3, 1.0, 0.7, 1.0, 1.0, 0.0);
    c.bench_function("cubic_bezier_sample_1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                black_box(curve.next_vector(i as f32 / 1000.0));
            }
        });
    });
}

fn bench_arc_sample_1000(c: &mut Criterion) {
    let arc = CircleArc::new(0.0, 0.0, 1.0, 1.0, 0.0, std::f32::consts::TAU);
    c.bench_function("arc_sample_1000", |b| {
        b.iter(|| {
            for i in 0..1000 {
                black_box(arc.next_vector(i as f32 / 1000.0));
            }
        });
    });
}

fn bench_total_length_100_shapes(c: &mut Criterion) {
    let shapes: Vec<Box<dyn Shape>> = (0..100)
        .map(|i| {
            let f = i as f32 / 100.0;
            if i % 3 == 0 {
                Box::new(Line::new_2d(f, f, f + 0.1, f + 0.1)) as Box<dyn Shape>
            } else if i % 3 == 1 {
                Box::new(CubicBezierCurve::new(f, f, f + 0.1, f + 0.2, f + 0.2, f + 0.1, f + 0.3, f))
                    as Box<dyn Shape>
            } else {
                Box::new(CircleArc::new(f, f, 0.1, 0.1, 0.0, std::f32::consts::TAU))
                    as Box<dyn Shape>
            }
        })
        .collect();

    c.bench_function("total_length_100_shapes", |b| {
        b.iter(|| black_box(total_length(&shapes)));
    });
}

fn bench_point_rotate(c: &mut Criterion) {
    let mut point = Point::new(0.5, 0.3, 0.1);
    c.bench_function("point_rotate", |b| {
        b.iter(|| {
            for _ in 0..1000 {
                point.rotate(0.1, 0.2, 0.3);
                black_box(&point);
            }
        });
    });
}

criterion_group!(
    benches,
    bench_line_sample_1000,
    bench_cubic_bezier_sample_1000,
    bench_arc_sample_1000,
    bench_total_length_100_shapes,
    bench_point_rotate,
);
criterion_main!(benches);
