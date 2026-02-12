use osci_core::shape::{normalize_shapes, Line, Shape};
use serde::Deserialize;

/// Parsed GPLA animation data: a sequence of frames, each containing drawable shapes.
pub struct GplaFrames {
    pub frames: Vec<Vec<Box<dyn Shape>>>,
    pub frame_rate: u32,
}

// ---------------------------------------------------------------------------
// JSON format structs
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct GplaJson {
    frames: Vec<GplaJsonFrame>,
}

#[derive(Deserialize)]
struct GplaJsonFrame {
    objects: Vec<GplaJsonObject>,
    #[serde(rename = "focalLength")]
    focal_length: f64,
}

#[derive(Deserialize)]
struct GplaJsonObject {
    vertices: Vec<Vec<GplaJsonVertex>>,
    matrix: Vec<f64>,
}

#[derive(Deserialize)]
struct GplaJsonVertex {
    x: f64,
    y: f64,
    z: f64,
}

// ---------------------------------------------------------------------------
// Internal types used during parsing
// ---------------------------------------------------------------------------

/// A 3D vertex used during frame assembly.
#[derive(Clone, Copy)]
struct Vertex3 {
    x: f64,
    y: f64,
    z: f64,
}

/// A single stroke (polyline) of 3D vertices.
type Stroke = Vec<Vertex3>;

/// An object with its strokes and 4x4 transform matrix.
struct GplaObject {
    strokes: Vec<Stroke>,
    matrix: [f64; 16],
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse GPLA data, auto-detecting binary vs JSON format.
pub fn parse_gpla(data: &[u8]) -> Result<GplaFrames, String> {
    // Trim leading whitespace to detect JSON
    let trimmed = data.iter().position(|&b| !b.is_ascii_whitespace());
    if let Some(pos) = trimmed {
        let first = data[pos];
        if first == b'{' || first == b'[' {
            return parse_json_gpla(data);
        }
    }

    // Check for binary GPLA header
    if data.len() >= 8 {
        let tag = read_tag(&data[..8]);
        if tag == "GPLA    " {
            return parse_binary_gpla(data);
        }
    }

    Err("Unrecognised GPLA format: data is neither JSON nor valid binary GPLA".to_string())
}

// ---------------------------------------------------------------------------
// JSON parser
// ---------------------------------------------------------------------------

fn parse_json_gpla(data: &[u8]) -> Result<GplaFrames, String> {
    let gpla: GplaJson =
        serde_json::from_slice(data).map_err(|e| format!("Failed to parse GPLA JSON: {e}"))?;

    let mut frames = Vec::with_capacity(gpla.frames.len());

    for json_frame in &gpla.frames {
        let focal_length = json_frame.focal_length;

        // Collect objects
        let objects: Vec<GplaObject> = json_frame
            .objects
            .iter()
            .map(|obj| {
                let strokes: Vec<Stroke> = obj
                    .vertices
                    .iter()
                    .map(|stroke_verts| {
                        stroke_verts
                            .iter()
                            .map(|v| Vertex3 {
                                x: v.x,
                                y: v.y,
                                z: v.z,
                            })
                            .collect()
                    })
                    .collect();

                let mut matrix = [0.0f64; 16];
                for (i, val) in obj.matrix.iter().enumerate().take(16) {
                    matrix[i] = *val;
                }

                GplaObject { strokes, matrix }
            })
            .collect();

        let mut shapes = assemble_frame(&objects, focal_length);
        if !shapes.is_empty() {
            normalize_shapes(&mut shapes);
        }
        frames.push(shapes);
    }

    // JSON format does not encode frame rate; use a sensible default.
    Ok(GplaFrames {
        frames,
        frame_rate: 24,
    })
}

// ---------------------------------------------------------------------------
// Binary parser
// ---------------------------------------------------------------------------

fn parse_binary_gpla(data: &[u8]) -> Result<GplaFrames, String> {
    let mut pos: usize = 0;

    let read_i64 = |pos: &mut usize| -> Result<i64, String> {
        if *pos + 8 > data.len() {
            return Err("Unexpected end of GPLA binary data".to_string());
        }
        let val = i64::from_le_bytes(data[*pos..*pos + 8].try_into().unwrap());
        *pos += 8;
        Ok(val)
    };

    let read_f64 = |pos: &mut usize| -> Result<f64, String> {
        let raw = read_i64(pos)?;
        Ok(f64::from_bits(raw as u64))
    };

    let read_tag_at = |pos: &mut usize| -> Result<String, String> {
        let raw = read_i64(pos)?;
        Ok(read_tag(&raw.to_le_bytes()))
    };

    // Header: "GPLA    "
    let header = read_tag_at(&mut pos)?;
    if header != "GPLA    " {
        return Err(format!("Invalid GPLA header: {:?}", header));
    }

    // Version
    let _version = read_i64(&mut pos)?;

    // FILE tag
    let file_tag = read_tag_at(&mut pos)?;
    if file_tag != "FILE    " {
        return Err(format!("Expected FILE tag, got {:?}", file_tag));
    }

    // Metadata: fCount, fRate
    let frame_count = read_i64(&mut pos)? as usize;
    let frame_rate = read_i64(&mut pos)? as u32;

    // DONE tag after metadata
    let done_tag = read_tag_at(&mut pos)?;
    if done_tag != "DONE    " {
        return Err(format!("Expected DONE after FILE metadata, got {:?}", done_tag));
    }

    // Parse frames
    let mut frames = Vec::with_capacity(frame_count);

    for _ in 0..frame_count {
        // FRAME tag
        let frame_tag = read_tag_at(&mut pos)?;
        if frame_tag != "FRAME   " {
            return Err(format!("Expected FRAME tag, got {:?}", frame_tag));
        }

        // Focal length
        let focal_length = read_f64(&mut pos)?;

        // OBJECTS tag
        let objects_tag = read_tag_at(&mut pos)?;
        if objects_tag != "OBJECTS " {
            return Err(format!("Expected OBJECTS tag, got {:?}", objects_tag));
        }

        let mut objects: Vec<GplaObject> = Vec::new();

        // Read objects until DONE
        loop {
            let tag = read_tag_at(&mut pos)?;
            if tag == "DONE    " {
                break;
            }
            if tag != "OBJECT  " {
                return Err(format!("Expected OBJECT or DONE, got {:?}", tag));
            }

            // MATRIX tag
            let matrix_tag = read_tag_at(&mut pos)?;
            if matrix_tag != "MATRIX  " {
                return Err(format!("Expected MATRIX tag, got {:?}", matrix_tag));
            }

            let mut matrix = [0.0f64; 16];
            for m in matrix.iter_mut() {
                *m = read_f64(&mut pos)?;
            }

            // STROKES tag
            let strokes_tag = read_tag_at(&mut pos)?;
            if strokes_tag != "STROKES " {
                return Err(format!("Expected STROKES tag, got {:?}", strokes_tag));
            }

            let mut strokes: Vec<Stroke> = Vec::new();

            // Read strokes until DONE
            loop {
                let tag = read_tag_at(&mut pos)?;
                if tag == "DONE    " {
                    break;
                }
                if tag != "STROKE  " {
                    return Err(format!("Expected STROKE or DONE, got {:?}", tag));
                }

                let vertex_count = read_i64(&mut pos)? as usize;

                // VERTICES tag
                let verts_tag = read_tag_at(&mut pos)?;
                if verts_tag != "VERTICES" {
                    return Err(format!("Expected VERTICES tag, got {:?}", verts_tag));
                }

                let mut stroke = Vec::with_capacity(vertex_count);
                for _ in 0..vertex_count {
                    let x = read_f64(&mut pos)?;
                    let y = read_f64(&mut pos)?;
                    let z = read_f64(&mut pos)?;
                    stroke.push(Vertex3 { x, y, z });
                }

                // DONE after vertices
                let done = read_tag_at(&mut pos)?;
                if done != "DONE    " {
                    return Err(format!(
                        "Expected DONE after VERTICES, got {:?}",
                        done
                    ));
                }

                strokes.push(stroke);
            }

            objects.push(GplaObject { strokes, matrix });
        }

        let mut shapes = assemble_frame(&objects, focal_length);
        if !shapes.is_empty() {
            normalize_shapes(&mut shapes);
        }
        frames.push(shapes);
    }

    Ok(GplaFrames { frames, frame_rate })
}

// ---------------------------------------------------------------------------
// Frame assembly helpers
// ---------------------------------------------------------------------------

/// Assemble a single frame: apply transforms, project, and create line shapes.
fn assemble_frame(objects: &[GplaObject], focal_length: f64) -> Vec<Box<dyn Shape>> {
    let mut shapes: Vec<Box<dyn Shape>> = Vec::new();

    for obj in objects {
        // Reorder strokes using nearest-neighbor greedy algorithm
        let reordered = reorder_strokes(&obj.strokes);

        for stroke in &reordered {
            if stroke.len() < 2 {
                continue;
            }

            for pair in stroke.windows(2) {
                let v0 = pair[0];
                let v1 = pair[1];

                let m = &obj.matrix;

                // Apply row-major 4x4 matrix
                let rx0 = v0.x * m[0] + v0.y * m[1] + v0.z * m[2] + m[3];
                let ry0 = v0.x * m[4] + v0.y * m[5] + v0.z * m[6] + m[7];
                let rz0 = v0.x * m[8] + v0.y * m[9] + v0.z * m[10] + m[11];

                let rx1 = v1.x * m[0] + v1.y * m[1] + v1.z * m[2] + m[3];
                let ry1 = v1.x * m[4] + v1.y * m[5] + v1.z * m[6] + m[7];
                let rz1 = v1.x * m[8] + v1.y * m[9] + v1.z * m[10] + m[11];

                // Only draw if both z values are < 0 (behind camera)
                if rz0 >= 0.0 || rz1 >= 0.0 {
                    continue;
                }

                // Perspective projection
                let px0 = (rx0 * focal_length / rz0) as f32;
                let py0 = (ry0 * focal_length / rz0) as f32;
                let px1 = (rx1 * focal_length / rz1) as f32;
                let py1 = (ry1 * focal_length / rz1) as f32;

                shapes.push(Box::new(Line::new_2d(px0, py0, px1, py1)));
            }
        }
    }

    shapes
}

/// Reorder strokes using nearest-neighbor (greedy) to minimise jumps between
/// consecutive strokes. Uses 3D Euclidean distance between the end of the
/// current stroke and the start of candidate strokes.
fn reorder_strokes(strokes: &[Stroke]) -> Vec<Stroke> {
    let n = strokes.len();
    if n <= 1 {
        return strokes.to_vec();
    }

    let mut visited = vec![false; n];
    let mut order = Vec::with_capacity(n);

    // Start with stroke 0
    visited[0] = true;
    order.push(strokes[0].clone());

    for _ in 1..n {
        let last_stroke = order.last().unwrap();
        let end = match last_stroke.last() {
            Some(v) => *v,
            None => Vertex3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        };

        let mut best_idx = 0;
        let mut best_dist = f64::MAX;

        for (j, stroke) in strokes.iter().enumerate() {
            if visited[j] {
                continue;
            }
            if let Some(start) = stroke.first() {
                let dx = end.x - start.x;
                let dy = end.y - start.y;
                let dz = end.z - start.z;
                let dist = (dx * dx + dy * dy + dz * dz).sqrt();
                if dist < best_dist {
                    best_dist = dist;
                    best_idx = j;
                }
            }
        }

        visited[best_idx] = true;
        order.push(strokes[best_idx].clone());
    }

    order
}

/// Read an 8-byte tag from raw bytes, interpreting them as ASCII characters.
fn read_tag(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(8);
    for &b in bytes.iter().take(8) {
        s.push(b as char);
    }
    s
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_single_frame() {
        let json = r#"{
            "frames": [
                {
                    "objects": [
                        {
                            "vertices": [
                                [
                                    {"x": 0.0, "y": 0.0, "z": -1.0},
                                    {"x": 1.0, "y": 1.0, "z": -1.0}
                                ]
                            ],
                            "matrix": [
                                1,0,0,0,
                                0,1,0,0,
                                0,0,1,0,
                                0,0,0,1
                            ]
                        }
                    ],
                    "focalLength": 1.0
                }
            ]
        }"#;

        let result = parse_gpla(json.as_bytes()).unwrap();
        assert_eq!(result.frames.len(), 1);
        // Identity matrix, z = -1, so projection = x * 1 / (-1) = -x
        assert!(!result.frames[0].is_empty());
    }

    #[test]
    fn test_parse_json_multiple_frames() {
        let json = r#"{
            "frames": [
                {
                    "objects": [
                        {
                            "vertices": [
                                [
                                    {"x": 0.0, "y": 0.0, "z": -2.0},
                                    {"x": 1.0, "y": 0.0, "z": -2.0}
                                ]
                            ],
                            "matrix": [1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]
                        }
                    ],
                    "focalLength": 1.5
                },
                {
                    "objects": [
                        {
                            "vertices": [
                                [
                                    {"x": 0.0, "y": 0.0, "z": -3.0},
                                    {"x": 2.0, "y": 1.0, "z": -3.0}
                                ]
                            ],
                            "matrix": [1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]
                        }
                    ],
                    "focalLength": 2.0
                }
            ]
        }"#;

        let result = parse_gpla(json.as_bytes()).unwrap();
        assert_eq!(result.frames.len(), 2);
        assert!(!result.frames[0].is_empty());
        assert!(!result.frames[1].is_empty());
    }

    #[test]
    fn test_positive_z_filtered_out() {
        // Vertices with z >= 0 should not produce any lines
        let json = r#"{
            "frames": [
                {
                    "objects": [
                        {
                            "vertices": [
                                [
                                    {"x": 0.0, "y": 0.0, "z": 1.0},
                                    {"x": 1.0, "y": 1.0, "z": 1.0}
                                ]
                            ],
                            "matrix": [1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]
                        }
                    ],
                    "focalLength": 1.0
                }
            ]
        }"#;

        let result = parse_gpla(json.as_bytes()).unwrap();
        assert_eq!(result.frames.len(), 1);
        assert!(result.frames[0].is_empty());
    }

    #[test]
    fn test_reorder_strokes_identity() {
        // Single stroke should remain unchanged
        let strokes = vec![vec![
            Vertex3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Vertex3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
        ]];
        let reordered = reorder_strokes(&strokes);
        assert_eq!(reordered.len(), 1);
    }

    #[test]
    fn test_reorder_strokes_nearest_neighbor() {
        // stroke 0: (0,0,0) -> (1,0,0)
        // stroke 1: (10,10,10) -> (11,10,10)   far from stroke 0 end
        // stroke 2: (1.1,0,0) -> (2,0,0)       close to stroke 0 end
        let strokes = vec![
            vec![
                Vertex3 { x: 0.0, y: 0.0, z: 0.0 },
                Vertex3 { x: 1.0, y: 0.0, z: 0.0 },
            ],
            vec![
                Vertex3 { x: 10.0, y: 10.0, z: 10.0 },
                Vertex3 { x: 11.0, y: 10.0, z: 10.0 },
            ],
            vec![
                Vertex3 { x: 1.1, y: 0.0, z: 0.0 },
                Vertex3 { x: 2.0, y: 0.0, z: 0.0 },
            ],
        ];

        let reordered = reorder_strokes(&strokes);
        assert_eq!(reordered.len(), 3);

        // After stroke 0, stroke 2 should come next (nearest to (1,0,0))
        let second_start = reordered[1].first().unwrap();
        assert!((second_start.x - 1.1).abs() < 0.001);
    }

    #[test]
    fn test_auto_detect_json() {
        let json = b"  { \"frames\": [] }";
        let result = parse_gpla(json);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().frames.len(), 0);
    }

    #[test]
    fn test_auto_detect_invalid() {
        let garbage = b"NOT_GPLA";
        let result = parse_gpla(garbage);
        assert!(result.is_err());
    }

    #[test]
    fn test_matrix_transform() {
        // Use a scaling matrix that doubles x and y
        let json = r#"{
            "frames": [
                {
                    "objects": [
                        {
                            "vertices": [
                                [
                                    {"x": 0.0, "y": 0.0, "z": -1.0},
                                    {"x": 1.0, "y": 0.0, "z": -1.0}
                                ]
                            ],
                            "matrix": [2,0,0,0, 0,2,0,0, 0,0,1,0, 0,0,0,1]
                        }
                    ],
                    "focalLength": 1.0
                }
            ]
        }"#;

        let result = parse_gpla(json.as_bytes()).unwrap();
        assert_eq!(result.frames.len(), 1);
        assert!(!result.frames[0].is_empty());
    }

    #[test]
    fn test_empty_strokes_ignored() {
        let json = r#"{
            "frames": [
                {
                    "objects": [
                        {
                            "vertices": [
                                [{"x": 0.0, "y": 0.0, "z": -1.0}]
                            ],
                            "matrix": [1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]
                        }
                    ],
                    "focalLength": 1.0
                }
            ]
        }"#;

        let result = parse_gpla(json.as_bytes()).unwrap();
        assert_eq!(result.frames.len(), 1);
        // A stroke with only 1 vertex produces no line segments
        assert!(result.frames[0].is_empty());
    }
}
