use osci_core::shape::{normalize_shapes, CubicBezierCurve, Line, QuadraticBezierCurve, Shape};

/// Parse SVG data into a vector of drawable shapes.
///
/// The SVG is parsed using `usvg`, and all path segments are converted to
/// osci-core shape primitives. Y coordinates are negated to flip the SVG
/// coordinate system (Y-down) into the oscilloscope coordinate system (Y-up).
/// The resulting shapes are normalized to fit within [-1, 1].
pub fn parse_svg(data: &[u8]) -> Result<Vec<Box<dyn Shape>>, String> {
    let tree = usvg::Tree::from_data(data, &usvg::Options::default())
        .map_err(|e| format!("Failed to parse SVG: {e}"))?;

    let mut shapes: Vec<Box<dyn Shape>> = Vec::new();
    collect_shapes_from_group(&tree.root, &mut shapes);

    if !shapes.is_empty() {
        normalize_shapes(&mut shapes);
    }

    Ok(shapes)
}

/// Recursively walk a usvg Group node, collecting shapes from all Path children.
fn collect_shapes_from_group(group: &usvg::Group, shapes: &mut Vec<Box<dyn Shape>>) {
    for child in &group.children {
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
fn collect_shapes_from_path(path: &usvg::Path, shapes: &mut Vec<Box<dyn Shape>>) {
    let transform = path.abs_transform;

    let mut cur_x: f64 = 0.0;
    let mut cur_y: f64 = 0.0;
    let mut subpath_start_x: f64 = 0.0;
    let mut subpath_start_y: f64 = 0.0;

    for segment in path.data.segments() {
        use usvg::tiny_skia_path::PathSegment;
        match segment {
            PathSegment::MoveTo(pt) => {
                let (tx, ty) = transform.map_point(pt.x as f64, pt.y as f64);
                cur_x = tx;
                cur_y = ty;
                subpath_start_x = tx;
                subpath_start_y = ty;
            }
            PathSegment::LineTo(pt) => {
                let (tx, ty) = transform.map_point(pt.x as f64, pt.y as f64);
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
                let (tx1, ty1) = transform.map_point(pt1.x as f64, pt1.y as f64);
                let (tx2, ty2) = transform.map_point(pt2.x as f64, pt2.y as f64);
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
                let (tx1, ty1) = transform.map_point(pt1.x as f64, pt1.y as f64);
                let (tx2, ty2) = transform.map_point(pt2.x as f64, pt2.y as f64);
                let (tx3, ty3) = transform.map_point(pt3.x as f64, pt3.y as f64);
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
}
