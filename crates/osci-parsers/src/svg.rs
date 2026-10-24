use osci_core::shape::{CubicBezierCurve, Line, QuadraticBezierCurve, Shape};

/// Parse SVG data into a vector of drawable shapes.
///
/// The SVG is parsed using `usvg`, and all path segments are converted to
/// osci-core shape primitives. Y coordinates are negated to flip the SVG
/// coordinate system (Y-down) into the oscilloscope coordinate system (Y-up).
/// The resulting shapes are centred and scaled to fit within [-0.9, 0.9].
pub fn parse_svg(data: &[u8]) -> Result<Vec<Box<dyn Shape>>, String> {
    let tree = usvg::Tree::from_data(data, &usvg::Options::default())
        .map_err(|e| format!("Failed to parse SVG: {e}"))?;

    let mut shapes: Vec<Box<dyn Shape>> = Vec::new();
    collect_shapes_from_group(tree.root(), &mut shapes);

    fit_centered(&mut shapes);

    Ok(shapes)
}

/// Centre shapes on the origin and scale them uniformly so the longer side
/// spans -0.9..0.9. Port of the `normalise` step in osci-render's
/// `SvgParser::pathToShapes`, which bounds by shape endpoints.
///
/// Unlike `normalize_shapes`, this does not flip Y, so callers must already
/// have converted to Y-up.
pub fn fit_centered(shapes: &mut [Box<dyn Shape>]) {
    if shapes.is_empty() {
        return;
    }
    let (mut min_x, mut min_y) = (f32::MAX, f32::MAX);
    let (mut max_x, mut max_y) = (f32::MIN, f32::MIN);
    for shape in shapes.iter() {
        for p in [shape.next_vector(0.0), shape.next_vector(1.0)] {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
    }
    let size = (max_x - min_x).max(max_y - min_y);
    if !size.is_finite() || size <= 0.0 {
        return; // A single point (or bad data); nothing to scale.
    }
    let (cx, cy, scale) = ((min_x + max_x) / 2.0, (min_y + max_y) / 2.0, 1.8 / size);
    for shape in shapes.iter_mut() {
        shape.translate(-cx, -cy, 0.0);
        shape.scale(scale, scale, 1.0);
    }
}

/// Recursively walk a usvg Group node, collecting shapes from all Path children.
fn collect_shapes_from_group(group: &usvg::Group, shapes: &mut Vec<Box<dyn Shape>>) {
    for child in group.children() {
        match child {
            usvg::Node::Group(ref g) => {
                collect_shapes_from_group(g, shapes);
            }
            usvg::Node::Path(ref path) => {
                collect_shapes_from_path(path, shapes);
            }
            _ => {}
        }
    }
}

/// Convert a usvg Path into osci-core shapes by iterating over its path segments.
///
/// Each segment is transformed by the path's absolute transform before being
/// converted. Y coordinates are negated to flip from SVG's Y-down to Y-up.
/// Apply the path's absolute transform to a tiny-skia Point, returning transformed (x, y) as f64.
fn transform_point(transform: &usvg::Transform, x: f32, y: f32) -> (f64, f64) {
    let mut pt = usvg::tiny_skia_path::Point { x, y };
    transform.map_point(&mut pt);
    (pt.x as f64, pt.y as f64)
}

fn collect_shapes_from_path(path: &usvg::Path, shapes: &mut Vec<Box<dyn Shape>>) {
    let transform = path.abs_transform();

    let mut cur_x: f64 = 0.0;
    let mut cur_y: f64 = 0.0;
    let mut subpath_start_x: f64 = 0.0;
    let mut subpath_start_y: f64 = 0.0;

    for segment in path.data().segments() {
        use usvg::tiny_skia_path::PathSegment;
        match segment {
            PathSegment::MoveTo(pt) => {
                let (tx, ty) = transform_point(&transform, pt.x, pt.y);
                cur_x = tx;
                cur_y = ty;
                subpath_start_x = tx;
                subpath_start_y = ty;
            }
            PathSegment::LineTo(pt) => {
                let (tx, ty) = transform_point(&transform, pt.x, pt.y);
                shapes.push(Box::new(Line::new_2d(
                    cur_x as f32,
                    -cur_y as f32,
                    tx as f32,
                    -ty as f32,
                )));
                cur_x = tx;
                cur_y = ty;
            }
            PathSegment::QuadTo(pt1, pt2) => {
                let (tx1, ty1) = transform_point(&transform, pt1.x, pt1.y);
                let (tx2, ty2) = transform_point(&transform, pt2.x, pt2.y);
                shapes.push(Box::new(QuadraticBezierCurve::new(
                    cur_x as f32,
                    -cur_y as f32,
                    tx1 as f32,
                    -ty1 as f32,
                    tx2 as f32,
                    -ty2 as f32,
                )));
                cur_x = tx2;
                cur_y = ty2;
            }
            PathSegment::CubicTo(pt1, pt2, pt3) => {
                let (tx1, ty1) = transform_point(&transform, pt1.x, pt1.y);
                let (tx2, ty2) = transform_point(&transform, pt2.x, pt2.y);
                let (tx3, ty3) = transform_point(&transform, pt3.x, pt3.y);
                shapes.push(Box::new(CubicBezierCurve::new(
                    cur_x as f32,
                    -cur_y as f32,
                    tx1 as f32,
                    -ty1 as f32,
                    tx2 as f32,
                    -ty2 as f32,
                    tx3 as f32,
                    -ty3 as f32,
                )));
                cur_x = tx3;
                cur_y = ty3;
            }
            PathSegment::Close => {
                // Close path: draw a line back to the subpath start if we're not already there
                let dx = cur_x - subpath_start_x;
                let dy = cur_y - subpath_start_y;
                if (dx * dx + dy * dy).sqrt() > 1e-6 {
                    shapes.push(Box::new(Line::new_2d(
                        cur_x as f32,
                        -cur_y as f32,
                        subpath_start_x as f32,
                        -subpath_start_y as f32,
                    )));
                }
                cur_x = subpath_start_x;
                cur_y = subpath_start_y;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_svg() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <line x1="0" y1="0" x2="100" y2="100"/>
        </svg>"#;
        let shapes = parse_svg(svg).unwrap();
        assert!(!shapes.is_empty());
    }

    #[test]
    fn test_parse_svg_rect() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
            <rect x="10" y="10" width="80" height="80"/>
        </svg>"#;
        let shapes = parse_svg(svg).unwrap();
        assert!(shapes.len() >= 4); // rect = 4 lines
    }

    /// Start point of each shape, rounded to 3 decimals.
    fn starts(shapes: &[Box<dyn Shape>]) -> Vec<(f32, f32)> {
        shapes
            .iter()
            .map(|s| s.next_vector(0.0))
            .map(|p| ((p.x * 1e3).round() / 1e3, (p.y * 1e3).round() / 1e3))
            .collect()
    }

    #[test]
    fn svg_is_upright_and_centred() {
        // Square at 10..90 in a 0..100 canvas, drawn from the top-left corner.
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
            <path d="M 10 10 L 90 10 L 90 90 L 10 90 Z"/></svg>"#;
        let shapes = parse_svg(svg).unwrap();
        // Y points up, so the SVG's top-left is (-, +). Fitted to 0.9 like osci-render.
        assert_eq!(starts(&shapes), vec![(-0.9, 0.9), (0.9, 0.9), (0.9, -0.9), (-0.9, -0.9)]);
    }

    #[test]
    fn wide_svg_keeps_its_proportions() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 100">
            <path d="M 50 25 L 250 25 L 250 75 L 50 75 Z"/></svg>"#;
        let shapes = parse_svg(svg).unwrap();
        // 200 x 50 box: x fills -0.9..0.9, y a quarter of that, centred.
        assert_eq!(starts(&shapes), vec![(-0.9, 0.225), (0.9, 0.225), (0.9, -0.225), (-0.9, -0.225)]);
    }
}
