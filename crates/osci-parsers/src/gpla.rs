//! GPLA line art, as exported by osci-render's Blender add-on.
//!
//! Port of `LineArtParser` from osci-render (osci_file_import). Frames are
//! projected through the camera and used as-is: Blender writes a negative
//! focal length (`-0.05 * lens`), so `x * focal / z` comes out upright and
//! a 50 mm camera's view spans roughly -0.9..0.9. Frames are not normalised,
//! so the camera's framing holds across an animation.

use osci_core::shape::{Line, Shape};
use serde::Deserialize;

/// Parsed GPLA animation data: a sequence of frames, each containing drawable shapes.
pub struct GplaFrames {
    pub frames: Vec<Vec<Box<dyn Shape>>>,
    pub frame_rate: u32,
}

/// Frame rate used when the file doesn't give one.
const DEFAULT_FRAME_RATE: u32 = 24;

// ---------------------------------------------------------------------------
// JSON format structs
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct GplaJson {
    frames: Vec<GplaJsonFrame>,
}

#[derive(Deserialize)]
struct GplaJsonFrame {
    #[serde(default)]
    objects: Vec<GplaJsonObject>,
    #[serde(rename = "focalLength")]
    focal_length: Option<f64>,
}

#[derive(Deserialize)]
struct GplaJsonObject {
    #[serde(default)]
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

    if data.starts_with(GPLA) {
        return parse_binary_gpla(data);
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
        // Like osci-render, skip frames with no objects or no focal length.
        let Some(focal_length) = json_frame.focal_length else {
            continue;
        };
        if json_frame.objects.is_empty() {
            continue;
        }

        let objects = json_objects(&json_frame.objects);
        frames.push(assemble_frame(&objects, focal_length));
    }

    // JSON format does not encode frame rate.
    Ok(GplaFrames {
        frames,
        frame_rate: DEFAULT_FRAME_RATE,
    })
}

/// Parse one frame of the older JSON streaming format,
/// `{"objects": [...], "focalLength": f}`, which Blender add-ons before
/// GPLA 2 sent over the live connection. Like osci-render, every object needs
/// a 16-value matrix and every stroke at least one vertex; a missing focal
/// length defaults to 1.
pub fn parse_json_frame(data: &[u8]) -> Result<Vec<Box<dyn Shape>>, String> {
    #[derive(Deserialize)]
    struct JsonFrame {
        objects: Vec<GplaJsonObject>,
        #[serde(rename = "focalLength", default = "default_focal_length")]
        focal_length: f64,
    }
    fn default_focal_length() -> f64 {
        1.0
    }

    let frame: JsonFrame = serde_json::from_slice(data)
        .map_err(|e| format!("Failed to parse GPLA JSON frame: {e}"))?;
    for obj in &frame.objects {
        if obj.matrix.len() != 16 {
            return Err(format!(
                "GPLA object matrix has {} values, expected 16",
                obj.matrix.len()
            ));
        }
        if obj.vertices.iter().any(Vec::is_empty) {
            return Err("GPLA object has an empty stroke".to_string());
        }
    }
    Ok(assemble_frame(
        &json_objects(&frame.objects),
        frame.focal_length,
    ))
}

fn json_objects(objects: &[GplaJsonObject]) -> Vec<GplaObject> {
    objects
        .iter()
        .map(|obj| {
            let strokes = obj
                .vertices
                .iter()
                .map(|stroke| {
                    stroke
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
        .collect()
}

// ---------------------------------------------------------------------------
// Binary parser
// ---------------------------------------------------------------------------
//
// The file is a sequence of little-endian 8-byte words. Tags are 8 ASCII
// characters padded with spaces; sections nest, and each `DONE` closes the
// innermost one. What the Blender add-on writes (GPLA 2.0.0):
//
//   GPLA <major> <minor> <patch>
//   FILE fCount <n> fRate <fps> DONE
//   FRAME focalLen <f64> OBJECTS
//     OBJECT MATRIX <16 x f64> DONE
//       STROKES
//         STROKE vertexCt <n> VERTICES <n x 3 x f64> DONE DONE
//       DONE
//     DONE
//   DONE DONE
//   ...more FRAMEs...
//   END GPLA
//
// Like osci-render, unknown tags are skipped, so later versions can add
// fields.

type Tag = [u8; 8];

const GPLA: &Tag = b"GPLA    ";
const FILE: &Tag = b"FILE    ";
const F_RATE: &Tag = b"fRate   ";
const DONE: &Tag = b"DONE    ";
const FRAME: &Tag = b"FRAME   ";
const FOCAL_LEN: &Tag = b"focalLen";
const OBJECTS: &Tag = b"OBJECTS ";
const OBJECT: &Tag = b"OBJECT  ";
const MATRIX: &Tag = b"MATRIX  ";
const STROKES: &Tag = b"STROKES ";
const STROKE: &Tag = b"STROKE  ";
const VERTEX_COUNT: &Tag = b"vertexCt";
const VERTICES: &Tag = b"VERTICES";
const END: &Tag = b"END GPLA";

/// Reads 8-byte words, failing cleanly at the end of the data.
struct Words<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Words<'_> {
    fn next(&mut self) -> Result<Tag, String> {
        let word = self
            .data
            .get(self.pos..self.pos + 8)
            .ok_or("Unexpected end of GPLA binary data")?;
        self.pos += 8;
        Ok(word.try_into().unwrap())
    }

    fn u64(&mut self) -> Result<u64, String> {
        Ok(u64::from_le_bytes(self.next()?))
    }

    fn f64(&mut self) -> Result<f64, String> {
        Ok(f64::from_le_bytes(self.next()?))
    }

    fn remaining(&self) -> usize {
        (self.data.len() - self.pos) / 8
    }

    fn expect(&mut self, tag: &Tag) -> Result<(), String> {
        let got = self.next()?;
        if &got != tag {
            return Err(format!(
                "Expected {:?}, got {:?}",
                tag_str(tag),
                tag_str(&got)
            ));
        }
        Ok(())
    }
}

fn tag_str(tag: &Tag) -> String {
    String::from_utf8_lossy(tag).into_owned()
}

fn parse_binary_gpla(data: &[u8]) -> Result<GplaFrames, String> {
    let mut r = Words { data, pos: 0 };
    r.expect(GPLA)?;
    for _ in 0..3 {
        r.next()?; // version: major, minor, patch
    }

    r.expect(FILE)?;
    let mut frame_rate = 0;
    loop {
        let tag = r.next()?;
        if &tag == DONE {
            break;
        }
        let value = r.u64()?;
        if &tag == F_RATE {
            frame_rate = value;
        }
        // fCount is informational; frames are counted as they're read.
    }

    let mut frames = Vec::new();
    loop {
        let tag = r.next()?;
        if &tag == END {
            break;
        }
        if &tag == FRAME {
            frames.push(parse_binary_frame(&mut r)?);
        }
        // Anything else, including the DONE that closes each FRAME, is skipped.
    }

    let frame_rate = match u32::try_from(frame_rate) {
        Ok(0) | Err(_) => DEFAULT_FRAME_RATE,
        Ok(fps) => fps,
    };
    Ok(GplaFrames { frames, frame_rate })
}

fn parse_binary_frame(r: &mut Words) -> Result<Vec<Box<dyn Shape>>, String> {
    let mut focal_length = None;
    loop {
        let tag = r.next()?;
        if &tag == OBJECTS {
            break;
        }
        let value = r.f64()?;
        if &tag == FOCAL_LEN {
            focal_length = Some(value);
        }
    }
    let focal_length = focal_length.ok_or("GPLA frame has no focal length")?;

    let mut objects = Vec::new();
    loop {
        let tag = r.next()?;
        if &tag == DONE {
            break;
        }
        if &tag == OBJECT {
            objects.push(parse_binary_object(r)?);
        }
    }

    Ok(assemble_frame(&objects, focal_length))
}

fn parse_binary_object(r: &mut Words) -> Result<GplaObject, String> {
    let mut matrix = None;
    let mut strokes = Vec::new();
    loop {
        let tag = r.next()?;
        if &tag == DONE {
            break;
        }
        if &tag == MATRIX {
            let mut m = [0.0f64; 16];
            for v in m.iter_mut() {
                *v = r.f64()?;
            }
            r.next()?; // DONE closing MATRIX
            matrix = Some(m);
        } else if &tag == STROKES {
            loop {
                let tag = r.next()?;
                if &tag == DONE {
                    break;
                }
                if &tag == STROKE {
                    strokes.push(parse_binary_stroke(r)?);
                }
            }
        }
    }
    let matrix = matrix.ok_or("GPLA object has no MATRIX")?;
    Ok(GplaObject { strokes, matrix })
}

fn parse_binary_stroke(r: &mut Words) -> Result<Stroke, String> {
    let mut count = 0;
    let mut stroke = Vec::new();
    loop {
        let tag = r.next()?;
        if &tag == DONE {
            break;
        }
        if &tag == VERTEX_COUNT {
            count = r.u64()?;
        } else if &tag == VERTICES {
            // Check against what's left before allocating.
            if count > (r.remaining() / 3) as u64 {
                return Err(format!(
                    "GPLA stroke claims {count} vertices, more than the data holds"
                ));
            }
            stroke.reserve(count as usize);
            for _ in 0..count {
                let (x, y, z) = (r.f64()?, r.f64()?, r.f64()?);
                stroke.push(Vertex3 { x, y, z });
            }
            // Skip to the DONE closing VERTICES.
            while &r.next()? != DONE {}
        }
    }
    Ok(stroke)
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

                // Blender cameras look down -z: skip lines with either end
                // at or behind the camera.
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
            ],
            vec![
                Vertex3 {
                    x: 10.0,
                    y: 10.0,
                    z: 10.0,
                },
                Vertex3 {
                    x: 11.0,
                    y: 10.0,
                    z: 10.0,
                },
            ],
            vec![
                Vertex3 {
                    x: 1.1,
                    y: 0.0,
                    z: 0.0,
                },
                Vertex3 {
                    x: 2.0,
                    y: 0.0,
                    z: 0.0,
                },
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

    // -- Orientation and binary format ---------------------------------------

    /// Endpoints of every line in a frame, rounded to 1e-4.
    fn endpoints(frame: &[Box<dyn Shape>]) -> Vec<[f32; 4]> {
        let r = |v: f32| (v * 1e4).round() / 1e4;
        frame
            .iter()
            .map(|s| {
                let (a, b) = (s.next_vector(0.0), s.next_vector(1.0));
                [r(a.x), r(a.y), r(b.x), r(b.y)]
            })
            .collect()
    }

    const IDENTITY: [f64; 16] = [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 0.0, //
        0.0, 0.0, 0.0, 1.0,
    ];

    /// Focal length the Blender add-on writes for a 50 mm lens.
    const FOCAL_50MM: f64 = -0.05 * 50.0;

    type TestObject = ([f64; 16], Vec<Vec<[f64; 3]>>);

    /// A binary GPLA file laid out exactly as the Blender add-on writes it
    /// (`gpla_header` and `get_frame_info_binary` in blender/osci_render).
    fn binary_gpla(fps: u64, frames: &[(f64, Vec<TestObject>)]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(b"GPLA    ");
        for v in [2u64, 0, 0] {
            out.extend(v.to_le_bytes());
        }
        out.extend(b"FILE    fCount  ");
        out.extend((frames.len() as u64).to_le_bytes());
        out.extend(b"fRate   ");
        out.extend(fps.to_le_bytes());
        out.extend(b"DONE    ");
        for (focal, objects) in frames {
            out.extend(b"FRAME   focalLen");
            out.extend(focal.to_le_bytes());
            out.extend(b"OBJECTS ");
            for (matrix, strokes) in objects {
                out.extend(b"OBJECT  MATRIX  ");
                for v in matrix {
                    out.extend(v.to_le_bytes());
                }
                out.extend(b"DONE    STROKES ");
                for stroke in strokes {
                    out.extend(b"STROKE  vertexCt");
                    out.extend((stroke.len() as u64).to_le_bytes());
                    out.extend(b"VERTICES");
                    for p in stroke {
                        for v in p {
                            out.extend(v.to_le_bytes());
                        }
                    }
                    out.extend(b"DONE    DONE    ");
                }
                out.extend(b"DONE    DONE    ");
            }
            out.extend(b"DONE    DONE    ");
        }
        out.extend(b"END GPLA");
        out
    }

    /// An "L" 5 units in front of the camera: down the left side, then
    /// along the bottom, with its corner at the origin.
    fn l_shape() -> Vec<Vec<[f64; 3]>> {
        vec![vec![[0.0, 1.0, -5.0], [0.0, 0.0, -5.0], [0.5, 0.0, -5.0]]]
    }

    #[test]
    fn binary_frame_is_upright_and_not_rescaled() {
        let data = binary_gpla(30, &[(FOCAL_50MM, vec![(IDENTITY, l_shape())])]);
        let result = parse_gpla(&data).unwrap();
        assert_eq!(result.frame_rate, 30);
        assert_eq!(result.frames.len(), 1);
        // Projection: 2.5 * x / 5. Up stays up, right stays right, and the
        // origin stays at the centre.
        assert_eq!(
            endpoints(&result.frames[0]),
            vec![[0.0, 0.5, 0.0, 0.0], [0.0, 0.0, 0.25, 0.0]]
        );
    }

    #[test]
    fn json_frame_is_upright_and_not_rescaled() {
        let json = r#"{"frames":[{"objects":[{"name":"Line Art",
            "vertices":[[{"x":0,"y":1,"z":-5},{"x":0,"y":0,"z":-5},{"x":0.5,"y":0,"z":-5}]],
            "matrix":[1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]}],"focalLength":-2.5}]}"#;
        let result = parse_gpla(json.as_bytes()).unwrap();
        assert_eq!(
            endpoints(&result.frames[0]),
            vec![[0.0, 0.5, 0.0, 0.0], [0.0, 0.0, 0.25, 0.0]]
        );
    }

    #[test]
    fn binary_applies_object_matrix() {
        // Move the object 1 unit right in camera space.
        let mut matrix = IDENTITY;
        matrix[3] = 1.0;
        let data = binary_gpla(24, &[(FOCAL_50MM, vec![(matrix, l_shape())])]);
        let result = parse_gpla(&data).unwrap();
        assert_eq!(
            endpoints(&result.frames[0]),
            vec![[0.5, 0.5, 0.5, 0.0], [0.5, 0.0, 0.75, 0.0]]
        );
    }

    #[test]
    fn binary_reads_every_frame_object_and_stroke() {
        let two_strokes = vec![
            vec![[0.0, 0.0, -1.0], [1.0, 0.0, -1.0]],
            vec![[1.0, 1.0, -1.0], [0.0, 1.0, -1.0], [0.0, 0.5, -1.0]],
        ];
        let frames = [
            (
                FOCAL_50MM,
                vec![(IDENTITY, two_strokes.clone()), (IDENTITY, l_shape())],
            ),
            (FOCAL_50MM, vec![]),
            (FOCAL_50MM, vec![(IDENTITY, l_shape())]),
        ];
        let result = parse_gpla(&binary_gpla(12, &frames)).unwrap();
        assert_eq!(result.frame_rate, 12);
        let counts: Vec<usize> = result.frames.iter().map(Vec::len).collect();
        assert_eq!(counts, vec![1 + 2 + 2, 0, 2]);
    }

    #[test]
    fn binary_frames_keep_the_camera_framing() {
        // The same L at two distances: the nearer one should draw bigger,
        // not be rescaled to match.
        let near = vec![vec![[0.0, 1.0, -2.0], [0.0, 0.0, -2.0]]];
        let far = vec![vec![[0.0, 1.0, -4.0], [0.0, 0.0, -4.0]]];
        let data = binary_gpla(
            24,
            &[
                (FOCAL_50MM, vec![(IDENTITY, near)]),
                (FOCAL_50MM, vec![(IDENTITY, far)]),
            ],
        );
        let result = parse_gpla(&data).unwrap();
        assert_eq!(endpoints(&result.frames[0]), vec![[0.0, 1.25, 0.0, 0.0]]);
        assert_eq!(endpoints(&result.frames[1]), vec![[0.0, 0.625, 0.0, 0.0]]);
    }

    #[test]
    fn binary_skips_unknown_tags() {
        // Extra metadata in FILE and FRAME, as a later add-on version might write.
        let mut data = binary_gpla(30, &[(FOCAL_50MM, vec![(IDENTITY, l_shape())])]);
        let insert_after = |data: &mut Vec<u8>, tag: &[u8], extra: &[u8]| {
            let at = data.windows(8).position(|w| w == tag).unwrap() + 8;
            data.splice(at..at, extra.iter().copied());
        };
        let mut extra = b"author  ".to_vec();
        extra.extend(7u64.to_le_bytes());
        insert_after(&mut data, b"FILE    ", &extra);
        let mut extra = b"exposure".to_vec();
        extra.extend(1.5f64.to_le_bytes());
        insert_after(&mut data, b"FRAME   ", &extra);

        let result = parse_gpla(&data).unwrap();
        assert_eq!(result.frame_rate, 30);
        assert_eq!(result.frames[0].len(), 2);
    }

    #[test]
    fn binary_without_frame_rate_uses_default() {
        let mut data = binary_gpla(0, &[(FOCAL_50MM, vec![(IDENTITY, l_shape())])]);
        let at = data.windows(8).position(|w| w == b"fRate   ").unwrap();
        data.drain(at..at + 16);
        assert_eq!(parse_gpla(&data).unwrap().frame_rate, DEFAULT_FRAME_RATE);
    }

    #[test]
    fn truncated_binary_is_an_error_not_a_panic() {
        let data = binary_gpla(24, &[(FOCAL_50MM, vec![(IDENTITY, l_shape())])]);
        for len in 8..data.len() {
            assert!(
                parse_gpla(&data[..len]).is_err(),
                "prefix of {len} bytes parsed"
            );
        }
    }

    #[test]
    fn binary_rejects_oversized_vertex_count() {
        let mut data = binary_gpla(24, &[(FOCAL_50MM, vec![(IDENTITY, l_shape())])]);
        let at = data.windows(8).position(|w| w == b"vertexCt").unwrap() + 8;
        data[at..at + 8].copy_from_slice(&u64::MAX.to_le_bytes());
        let err = parse_gpla(&data).err().unwrap();
        assert!(err.contains("vertices"), "{err}");
    }

    #[test]
    fn json_skips_frames_without_focal_length_or_objects() {
        let json = r#"{"frames":[
            {"objects":[{"vertices":[[{"x":0,"y":0,"z":-1},{"x":1,"y":0,"z":-1}]],
                         "matrix":[1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]}]},
            {"objects":[],"focalLength":-2.5},
            {"objects":[{"vertices":[[{"x":0,"y":0,"z":-1},{"x":1,"y":0,"z":-1}]],
                         "matrix":[1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]}],"focalLength":-2.5}
        ]}"#;
        let result = parse_gpla(json.as_bytes()).unwrap();
        assert_eq!(result.frames.len(), 1);
        assert_eq!(endpoints(&result.frames[0]), vec![[0.0, 0.0, 2.5, 0.0]]);
    }

    #[test]
    fn json_stream_frame_is_upright() {
        let json = br#"{"objects":[{"name":"Line Art",
            "vertices":[[{"x":0,"y":1,"z":-5},{"x":0,"y":0,"z":-5},{"x":0.5,"y":0,"z":-5}]],
            "matrix":[1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]}],"focalLength":-2.5}"#;
        let frame = parse_json_frame(json).unwrap();
        assert_eq!(
            endpoints(&frame),
            vec![[0.0, 0.5, 0.0, 0.0], [0.0, 0.0, 0.25, 0.0]]
        );
    }

    #[test]
    fn json_stream_frame_defaults_focal_length_to_one() {
        let json = br#"{"objects":[{"vertices":[[{"x":0,"y":0,"z":-1},{"x":1,"y":0,"z":-1}]],
            "matrix":[1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]}]}"#;
        assert_eq!(
            endpoints(&parse_json_frame(json).unwrap()),
            vec![[0.0, 0.0, -1.0, 0.0]]
        );
    }

    #[test]
    fn json_stream_frame_rejects_bad_objects() {
        let short_matrix =
            br#"{"objects":[{"vertices":[[{"x":0,"y":0,"z":-1}]],"matrix":[1,0,0]}]}"#;
        let empty_stroke = br#"{"objects":[{"vertices":[[]],
            "matrix":[1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,0,1]}]}"#;
        for bad in [&short_matrix[..], &empty_stroke[..], b"{}", b"not json"] {
            assert!(
                parse_json_frame(bad).is_err(),
                "{}",
                String::from_utf8_lossy(bad)
            );
        }
    }
}
