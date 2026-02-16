use osci_parsers::{parse_file, default_shapes, FileType, ParseResult};

// ── Helpers ──────────────────────────────────────────────────────

/// Check that all shapes in a collection produce bounded points in [-bound, bound].
fn shapes_bounded(shapes: &[Box<dyn osci_core::shape::Shape>], bound: f32) -> bool {
    for shape in shapes {
        for i in 0..=10 {
            let t = i as f32 / 10.0;
            let p = shape.next_vector(t);
            if p.x.abs() > bound || p.y.abs() > bound {
                return false;
            }
        }
    }
    true
}

// ── Embedded test data ───────────────────────────────────────────

const SVG_RECT: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
  <rect x="10" y="10" width="80" height="80" fill="none" stroke="black"/>
</svg>"#;

const SVG_CIRCLE: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
  <circle cx="50" cy="50" r="40" fill="none" stroke="black"/>
</svg>"#;

const SVG_POLYLINE: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
  <polyline points="10,90 50,10 90,90" fill="none" stroke="black"/>
</svg>"#;

// ── 1. SVG parsing ───────────────────────────────────────────────

#[test]
fn svg_rect_produces_shapes() {
    let result = parse_file(SVG_RECT, "svg").expect("SVG rect should parse");
    if let ParseResult::Shapes(shapes) = result {
        assert!(!shapes.is_empty(), "SVG rect produced no shapes");
        assert!(
            shapes_bounded(&shapes, 1.5),
            "SVG rect shapes out of normalized bounds"
        );
    } else {
        panic!("expected ParseResult::Shapes from SVG");
    }
}

#[test]
fn svg_circle_produces_shapes() {
    let result = parse_file(SVG_CIRCLE, "svg").expect("SVG circle should parse");
    if let ParseResult::Shapes(shapes) = result {
        assert!(!shapes.is_empty(), "SVG circle produced no shapes");
        assert!(
            shapes_bounded(&shapes, 1.5),
            "SVG circle shapes out of normalized bounds"
        );
    } else {
        panic!("expected ParseResult::Shapes from SVG");
    }
}

#[test]
fn svg_polyline_produces_shapes() {
    let result = parse_file(SVG_POLYLINE, "svg").expect("SVG polyline should parse");
    if let ParseResult::Shapes(shapes) = result {
        assert!(!shapes.is_empty(), "SVG polyline produced no shapes");
        assert!(
            shapes_bounded(&shapes, 1.5),
            "SVG polyline shapes out of normalized bounds"
        );
    } else {
        panic!("expected ParseResult::Shapes from SVG");
    }
}

// ── 2. OBJ parsing ──────────────────────────────────────────────

#[test]
fn obj_triangle_from_fixture() {
    let obj_data = include_bytes!("fixtures/triangle.obj");
    let result = parse_file(obj_data, "obj").expect("OBJ triangle should parse");
    if let ParseResult::Shapes(shapes) = result {
        assert!(!shapes.is_empty(), "OBJ triangle produced no shapes");
        // Triangle has 3 edges
        assert!(shapes.len() >= 3, "OBJ triangle should have at least 3 edges, got {}", shapes.len());
        assert!(
            shapes_bounded(&shapes, 2.0),
            "OBJ triangle shapes out of normalized bounds"
        );
    } else {
        panic!("expected ParseResult::Shapes from OBJ");
    }
}

#[test]
fn obj_embedded_cube_edges() {
    let obj_data = b"\
v -1 -1 -1\n\
v  1 -1 -1\n\
v  1  1 -1\n\
v -1  1 -1\n\
v -1 -1  1\n\
v  1 -1  1\n\
v  1  1  1\n\
v -1  1  1\n\
f 1 2 3 4\n\
f 5 6 7 8\n\
f 1 2 6 5\n\
f 2 3 7 6\n\
f 3 4 8 7\n\
f 4 1 5 8\n";

    let result = parse_file(obj_data, "obj").expect("OBJ cube should parse");
    if let ParseResult::Shapes(shapes) = result {
        assert!(
            shapes.len() >= 12,
            "cube should produce at least 12 edges, got {}",
            shapes.len()
        );
    } else {
        panic!("expected ParseResult::Shapes from OBJ");
    }
}

// ── 3. Text parsing ─────────────────────────────────────────────

#[test]
fn text_parse_hello() {
    let result = parse_file(b"Hello", "txt").expect("text should parse");
    if let ParseResult::Shapes(shapes) = result {
        assert!(!shapes.is_empty(), "text 'Hello' produced no shapes");
    } else {
        panic!("expected ParseResult::Shapes from text");
    }
}

#[test]
fn text_empty_string() {
    let result = parse_file(b"", "txt").expect("empty text should parse");
    if let ParseResult::Shapes(shapes) = result {
        assert!(shapes.is_empty(), "empty text should produce no shapes");
    } else {
        panic!("expected ParseResult::Shapes from text");
    }
}

// ── 4. File dispatch ─────────────────────────────────────────────

#[test]
fn file_type_detection() {
    assert_eq!(FileType::from_extension("svg"), FileType::Svg);
    assert_eq!(FileType::from_extension("SVG"), FileType::Svg);
    assert_eq!(FileType::from_extension("obj"), FileType::Obj);
    assert_eq!(FileType::from_extension("txt"), FileType::Text);
    assert_eq!(FileType::from_extension("text"), FileType::Text);
    assert_eq!(FileType::from_extension("lua"), FileType::Lua);
    assert_eq!(FileType::from_extension("gpla"), FileType::Gpla);
    assert_eq!(FileType::from_extension("gif"), FileType::Gif);
    assert_eq!(FileType::from_extension("png"), FileType::Image);
    assert_eq!(FileType::from_extension("jpg"), FileType::Image);
    assert_eq!(FileType::from_extension("wav"), FileType::Audio);
    assert_eq!(FileType::from_extension("xyz"), FileType::Unknown);
}

#[test]
fn dispatch_svg_via_parse_file() {
    // Same SVG data, dispatched via extension
    let result = parse_file(SVG_RECT, "svg");
    assert!(result.is_ok(), "dispatch to SVG parser failed");
}

#[test]
fn dispatch_obj_via_parse_file() {
    let obj_data = include_bytes!("fixtures/triangle.obj");
    let result = parse_file(obj_data, "obj");
    assert!(result.is_ok(), "dispatch to OBJ parser failed");
}

#[test]
fn dispatch_unknown_returns_error() {
    let result = parse_file(b"data", "zzz");
    assert!(result.is_err(), "unknown extension should return error");
}

// ── 5. Default shapes ────────────────────────────────────────────

#[test]
fn default_shapes_is_square() {
    let shapes = default_shapes();
    assert_eq!(shapes.len(), 4, "default shapes should be 4 lines (square)");

    // All points should be within [-1, 1]
    assert!(
        shapes_bounded(&shapes, 1.0),
        "default shapes out of [-1, 1] bounds"
    );
}

#[test]
fn default_shapes_total_length_positive() {
    let shapes = default_shapes();
    let total: f32 = shapes.iter().map(|s| s.length()).sum();
    assert!(total > 0.0, "default shapes should have positive total length");
}

// ── 6. Error handling ────────────────────────────────────────────

#[test]
fn invalid_svg_returns_error() {
    let result = parse_file(b"not valid svg data at all", "svg");
    assert!(result.is_err(), "invalid SVG data should return Err");
}

#[test]
fn invalid_obj_returns_error_or_empty() {
    // tobj may return empty models or error for garbage
    let result = parse_file(b"garbage data for obj", "obj");
    match result {
        Err(_) => {} // expected
        Ok(ParseResult::Shapes(shapes)) => {
            // Some parsers produce empty results instead of erroring
            assert!(shapes.is_empty(), "garbage OBJ should produce no shapes");
        }
        _ => panic!("unexpected parse result for garbage OBJ"),
    }
}

#[test]
fn invalid_utf8_text_returns_error() {
    let bad_bytes: &[u8] = &[0xFF, 0xFE, 0x80, 0x81];
    let result = parse_file(bad_bytes, "txt");
    assert!(result.is_err(), "invalid UTF-8 should return Err for text format");
}
