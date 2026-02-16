use osci_core::Point;
use osci_effects::registry::build_registry;

// ── Helpers ──────────────────────────────────────────────────────

fn is_valid_point(p: Point) -> bool {
    p.x.is_finite() && p.y.is_finite() && p.z.is_finite()
        && p.r.is_finite() && p.g.is_finite() && p.b.is_finite()
}

/// Build a values slice with enough elements for any effect (padded with defaults).
fn padded_defaults(params: &[osci_core::EffectParameter]) -> Vec<f32> {
    let mut values: Vec<f32> = params.iter().map(|p| p.default_value).collect();
    // Pad to at least 8 values to handle effects that access beyond their declared params
    while values.len() < 8 {
        values.push(0.0);
    }
    values
}

const SAMPLE_RATE: f32 = 44100.0;
const FREQUENCY: f32 = 440.0;

fn test_input() -> Point {
    Point::with_rgb(0.5, -0.3, 0.7, 1.0, 0.8, 0.6)
}

// ── 1. Registry completeness ─────────────────────────────────────

#[test]
fn registry_has_27_effects() {
    let registry = build_registry();
    assert_eq!(registry.len(), 27, "expected 27 effects in registry");
}

#[test]
fn registry_ids_are_unique() {
    let registry = build_registry();
    let mut ids: Vec<&str> = registry.iter().map(|e| e.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 27, "duplicate effect IDs found");
}

#[test]
fn registry_names_are_non_empty() {
    let registry = build_registry();
    for entry in &registry {
        assert!(!entry.name.is_empty(), "effect '{}' has empty name", entry.id);
    }
}

#[test]
fn all_effects_constructible() {
    let registry = build_registry();
    for entry in &registry {
        let effect = (entry.constructor)();
        assert!(
            !effect.name().is_empty(),
            "effect '{}' returned empty name from trait",
            entry.id
        );
    }
}

// ── 2. Identity / passthrough with default params ────────────────

#[test]
fn all_effects_return_valid_point_with_defaults() {
    let registry = build_registry();
    let input = test_input();

    for entry in &registry {
        let mut effect = (entry.constructor)();
        let params = (entry.parameters)();
        let values = padded_defaults(&params);

        let output = effect.apply(0, input, Point::ZERO, &values, SAMPLE_RATE, FREQUENCY);
        assert!(
            is_valid_point(output),
            "effect '{}' returned invalid point with defaults: {:?}",
            entry.id,
            (output.x, output.y, output.z)
        );
    }
}

// ── 3. Signal modification ───────────────────────────────────────

#[test]
fn translate_moves_point() {
    let registry = build_registry();
    let entry = registry.iter().find(|e| e.id == "translate").unwrap();
    let mut effect = (entry.constructor)();
    // values: [translateX, translateY, translateZ]
    let values = vec![0.5, -0.2, 0.0];

    let input = Point::with_rgb(0.0, 0.0, 0.0, 1.0, 1.0, 1.0);
    let output = effect.apply(0, input, Point::ZERO, &values, SAMPLE_RATE, FREQUENCY);

    assert!(
        (output.x - 0.5).abs() < 0.01 && (output.y - (-0.2)).abs() < 0.01,
        "translate did not move point: ({}, {})",
        output.x,
        output.y
    );
}

#[test]
fn scale_scales_point() {
    let registry = build_registry();
    let entry = registry.iter().find(|e| e.id == "scale").unwrap();
    let mut effect = (entry.constructor)();
    // values: [scaleX, scaleY, scaleZ]
    let values = vec![2.0, 0.5, 1.0];

    let input = Point::with_rgb(0.4, 0.6, 0.0, 1.0, 1.0, 1.0);
    let output = effect.apply(0, input, Point::ZERO, &values, SAMPLE_RATE, FREQUENCY);

    assert!(
        (output.x - 0.8).abs() < 0.01,
        "scale X incorrect: {} expected 0.8",
        output.x
    );
    assert!(
        (output.y - 0.3).abs() < 0.01,
        "scale Y incorrect: {} expected 0.3",
        output.y
    );
}

#[test]
fn volume_attenuates() {
    let registry = build_registry();
    let entry = registry.iter().find(|e| e.id == "volume").unwrap();
    let mut effect = (entry.constructor)();
    let values = vec![0.5]; // half volume

    let input = Point::with_rgb(1.0, 1.0, 1.0, 1.0, 1.0, 1.0);
    let output = effect.apply(0, input, Point::ZERO, &values, SAMPLE_RATE, FREQUENCY);

    assert!(
        (output.x - 0.5).abs() < 0.01,
        "volume X incorrect: {} expected 0.5",
        output.x
    );
}

#[test]
fn rotate_z_rotates_xy() {
    let registry = build_registry();
    let entry = registry.iter().find(|e| e.id == "rotate").unwrap();
    let mut effect = (entry.constructor)();
    // values: [rotateX=0, rotateY=0, rotateZ=0.25 (quarter turn)]
    let values = vec![0.0, 0.0, 0.25];

    let input = Point::with_rgb(1.0, 0.0, 0.0, 1.0, 1.0, 1.0);
    let output = effect.apply(0, input, Point::ZERO, &values, SAMPLE_RATE, FREQUENCY);

    // After quarter-turn around Z, (1,0) should move significantly
    let distance = ((output.x - input.x).powi(2) + (output.y - input.y).powi(2)).sqrt();
    assert!(
        distance > 0.1,
        "rotate Z=0.25 did not move point enough: distance={}",
        distance
    );
}

// ── 4. Stateful effects — state evolves over time ────────────────

#[test]
fn smooth_state_evolves() {
    let registry = build_registry();
    let entry = registry.iter().find(|e| e.id == "smooth").unwrap();
    let mut effect = (entry.constructor)();
    // smooth amount = 0.9 (heavy smoothing)
    let values = vec![0.9];

    let input = Point::with_rgb(1.0, 1.0, 0.0, 1.0, 1.0, 1.0);

    // Process several samples — with heavy smoothing the output should
    // gradually approach the input, so early outputs should differ.
    let first = effect.apply(0, input, Point::ZERO, &values, SAMPLE_RATE, FREQUENCY);
    let mut last = first;
    for i in 1..100 {
        last = effect.apply(i, input, Point::ZERO, &values, SAMPLE_RATE, FREQUENCY);
    }

    // After 100 samples the smoothed value should be closer to the input
    // than the first sample was.
    let first_dist = (first.x - input.x).abs() + (first.y - input.y).abs();
    let last_dist = (last.x - input.x).abs() + (last.y - input.y).abs();
    assert!(
        last_dist <= first_dist + 0.001,
        "smooth did not converge: first_dist={first_dist}, last_dist={last_dist}"
    );
}

#[test]
fn delay_state_evolves() {
    let registry = build_registry();
    let entry = registry.iter().find(|e| e.id == "delay").unwrap();
    let mut effect = (entry.constructor)();
    // values: [decay=0.8, delayLength=0.01] (very short delay: 441 samples at 44100 Hz)
    let values = vec![0.8, 0.01];

    let input = Point::with_rgb(0.8, 0.6, 0.0, 1.0, 1.0, 1.0);
    let zero = Point::ZERO;

    // Feed input samples for longer than the delay length
    for i in 0..1000 {
        effect.apply(i, input, zero, &values, SAMPLE_RATE, FREQUENCY);
    }

    // Now feed zeros — the delay buffer should still produce non-zero output (echo)
    let mut found_echo = false;
    for i in 1000..2000 {
        let output = effect.apply(i, zero, zero, &values, SAMPLE_RATE, FREQUENCY);
        if output.x.abs() > 0.001 || output.y.abs() > 0.001 {
            found_echo = true;
            break;
        }
    }
    assert!(found_echo, "delay did not produce echo output after switching to zero input");
}

#[test]
fn bounce_state_evolves() {
    let registry = build_registry();
    let entry = registry.iter().find(|e| e.id == "bounce").unwrap();
    let mut effect = (entry.constructor)();
    let params = (entry.parameters)();
    let values = padded_defaults(&params);

    let input = Point::with_rgb(0.5, 0.5, 0.0, 1.0, 1.0, 1.0);

    let mut outputs = Vec::new();
    for i in 0..500 {
        let out = effect.apply(i, input, Point::ZERO, &values, SAMPLE_RATE, FREQUENCY);
        outputs.push(out);
    }

    // Check that bounce produces at least some variation in output
    let first = outputs[0];
    let has_variation = outputs.iter().any(|p| {
        (p.x - first.x).abs() > 0.001 || (p.y - first.y).abs() > 0.001
    });
    assert!(has_variation, "bounce produced no variation over 500 samples");
}

// ── 5. Determinism — same inputs produce same outputs ────────────

#[test]
fn all_effects_are_deterministic() {
    let registry = build_registry();
    let input = test_input();

    for entry in &registry {
        let params = (entry.parameters)();
        let values = padded_defaults(&params);

        // Run A
        let mut effect_a = (entry.constructor)();
        let mut outputs_a = Vec::new();
        for i in 0..10 {
            outputs_a.push(effect_a.apply(i, input, Point::ZERO, &values, SAMPLE_RATE, FREQUENCY));
        }

        // Run B (fresh instance)
        let mut effect_b = (entry.constructor)();
        let mut outputs_b = Vec::new();
        for i in 0..10 {
            outputs_b.push(effect_b.apply(i, input, Point::ZERO, &values, SAMPLE_RATE, FREQUENCY));
        }

        for (i, (a, b)) in outputs_a.iter().zip(outputs_b.iter()).enumerate() {
            assert!(
                (a.x - b.x).abs() < 1e-6 && (a.y - b.y).abs() < 1e-6 && (a.z - b.z).abs() < 1e-6,
                "effect '{}' is non-deterministic at sample {}: A=({},{},{}), B=({},{},{})",
                entry.id, i, a.x, a.y, a.z, b.x, b.y, b.z
            );
        }
    }
}

// ── 6. Parameter construction / validation ───────────────────────

#[test]
fn all_parameters_have_valid_ranges() {
    let registry = build_registry();

    for entry in &registry {
        let params = (entry.parameters)();
        assert!(
            !params.is_empty(),
            "effect '{}' has no parameters",
            entry.id
        );

        for param in &params {
            assert!(
                param.min <= param.default_value,
                "effect '{}' param '{}': min ({}) > default ({})",
                entry.id, param.name, param.min, param.default_value
            );
            assert!(
                param.default_value <= param.max,
                "effect '{}' param '{}': default ({}) > max ({})",
                entry.id, param.name, param.default_value, param.max
            );
            assert!(
                param.min < param.max,
                "effect '{}' param '{}': min ({}) >= max ({})",
                entry.id, param.name, param.min, param.max
            );
            assert!(
                !param.name.is_empty(),
                "effect '{}' has param with empty name",
                entry.id
            );
            assert!(
                !param.id.is_empty(),
                "effect '{}' has param with empty id",
                entry.id
            );
        }
    }
}

#[test]
fn find_effect_returns_known_ids() {
    use osci_effects::registry::find_effect;

    let known_ids = [
        "bitcrush", "bulge", "vectorCancelling", "ripple", "rotate",
        "translate", "scale", "swirl", "smooth", "delay", "dashedLine",
        "wobble", "duplicator", "multiplex", "unfold", "bounce", "twist",
        "skew", "polygonizer", "kaleidoscope", "vortex", "godRay",
        "spiralBitcrush", "perspective", "volume", "threshold", "frequency",
    ];

    for id in &known_ids {
        assert!(
            find_effect(id).is_some(),
            "find_effect('{}') returned None",
            id
        );
    }

    assert!(find_effect("nonexistent").is_none());
}
