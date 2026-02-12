use osci_core::shape::{Line, Shape};
use std::collections::HashSet;
use std::io::Cursor;

/// Parse OBJ (Wavefront) mesh data into a vector of drawable line shapes.
///
/// The mesh vertices are normalized by centering on the centroid and scaling
/// to fit within a reasonable range. Unique edges are extracted from all faces,
/// then reordered using a greedy nearest-neighbor heuristic to minimize jump
/// distances between consecutive edges (simplified Chinese Postman optimization).
pub fn parse_obj(data: &[u8]) -> Result<Vec<Box<dyn Shape>>, String> {
    let mut cursor = Cursor::new(data);

    let (models, _materials) = tobj::load_obj_buf(
        &mut cursor,
        &tobj::LoadOptions {
            triangulate: false,
            single_index: true,
            ..Default::default()
        },
        |_| Ok(Default::default()),
    )
    .map_err(|e| format!("Failed to parse OBJ: {e}"))?;

    // Collect all vertices and edges across all models
    let mut all_vertices: Vec<[f32; 3]> = Vec::new();
    let mut all_edges: HashSet<(u32, u32)> = HashSet::new();

    for model in &models {
        let mesh = &model.mesh;
        let positions = &mesh.positions;
        let vertex_offset = all_vertices.len() as u32;

        // Extract vertices (positions is [x0, y0, z0, x1, y1, z1, ...])
        let num_vertices = positions.len() / 3;
        for i in 0..num_vertices {
            all_vertices.push([
                positions[i * 3],
                positions[i * 3 + 1],
                positions[i * 3 + 2],
            ]);
        }

        // Extract edges from faces
        let indices = &mesh.indices;
        let face_arities = &mesh.face_arities;

        if face_arities.is_empty() {
            // All faces are triangles
            let num_faces = indices.len() / 3;
            for f in 0..num_faces {
                let base = f * 3;
                for j in 0..3 {
                    let a = indices[base + j] + vertex_offset;
                    let b = indices[base + (j + 1) % 3] + vertex_offset;
                    let edge = (a.min(b), a.max(b));
                    all_edges.insert(edge);
                }
            }
        } else {
            // Variable face arities
            let mut idx = 0;
            for &arity in face_arities.iter() {
                let n = arity as usize;
                for j in 0..n {
                    let a = indices[idx + j] + vertex_offset;
                    let b = indices[idx + (j + 1) % n] + vertex_offset;
                    let edge = (a.min(b), a.max(b));
                    all_edges.insert(edge);
                }
                idx += n;
            }
        }
    }

    if all_vertices.is_empty() || all_edges.is_empty() {
        return Ok(Vec::new());
    }

    // Normalize vertices: compute centroid, subtract it, then scale
    let num_verts = all_vertices.len() as f32;
    let mut cx = 0.0f32;
    let mut cy = 0.0f32;
    let mut cz = 0.0f32;
    for v in &all_vertices {
        cx += v[0];
        cy += v[1];
        cz += v[2];
    }
    cx /= num_verts;
    cy /= num_verts;
    cz /= num_verts;

    // Subtract centroid
    for v in all_vertices.iter_mut() {
        v[0] -= cx;
        v[1] -= cy;
        v[2] -= cz;
    }

    // Scale by 1.0 / (1.2 * max_distance_from_origin)
    let mut max_dist = 0.0f32;
    for v in &all_vertices {
        let dist = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if dist > max_dist {
            max_dist = dist;
        }
    }

    if max_dist > 0.0 {
        let scale = 1.0 / (1.2 * max_dist);
        for v in all_vertices.iter_mut() {
            v[0] *= scale;
            v[1] *= scale;
            v[2] *= scale;
        }
    }

    // Convert edges to a Vec for ordering
    let mut edges: Vec<(u32, u32)> = all_edges.into_iter().collect();

    // Reorder edges using greedy nearest-neighbor to minimize jump distances
    if edges.len() > 1 {
        edges = reorder_edges_nearest_neighbor(&edges, &all_vertices);
    }

    // Generate Line shapes
    let shapes: Vec<Box<dyn Shape>> = edges
        .iter()
        .map(|&(a, b)| {
            let va = &all_vertices[a as usize];
            let vb = &all_vertices[b as usize];
            Box::new(Line::new_3d(va[0], va[1], va[2], vb[0], vb[1], vb[2])) as Box<dyn Shape>
        })
        .collect();

    Ok(shapes)
}

/// Reorder edges using a greedy nearest-neighbor heuristic.
///
/// Starting from the first edge, repeatedly select the unvisited edge whose
/// endpoint is closest to the current position. Edges can be traversed in
/// either direction, so for each candidate we check the distance from the
/// current position to both the start and end of the candidate edge.
fn reorder_edges_nearest_neighbor(
    edges: &[(u32, u32)],
    vertices: &[[f32; 3]],
) -> Vec<(u32, u32)> {
    let n = edges.len();
    let mut used = vec![false; n];
    let mut result = Vec::with_capacity(n);

    // Start with the first edge
    used[0] = true;
    result.push(edges[0]);

    // Current endpoint is the second vertex of the first edge
    let mut cur_pos = vertices[edges[0].1 as usize];

    for _ in 1..n {
        let mut best_idx = 0;
        let mut best_dist = f32::MAX;
        let mut best_flip = false;

        for j in 0..n {
            if used[j] {
                continue;
            }

            let va = &vertices[edges[j].0 as usize];
            let vb = &vertices[edges[j].1 as usize];

            // Distance from current position to start of edge
            let dist_a = distance_sq(&cur_pos, va);
            // Distance from current position to end of edge
            let dist_b = distance_sq(&cur_pos, vb);

            if dist_a < best_dist {
                best_dist = dist_a;
                best_idx = j;
                best_flip = false;
            }
            if dist_b < best_dist {
                best_dist = dist_b;
                best_idx = j;
                best_flip = true;
            }
        }

        used[best_idx] = true;

        let edge = if best_flip {
            // Flip so we enter from the closer end
            (edges[best_idx].1, edges[best_idx].0)
        } else {
            edges[best_idx]
        };

        cur_pos = vertices[edge.1 as usize];
        result.push(edge);
    }

    result
}

/// Squared Euclidean distance between two 3D points (for comparison purposes).
#[inline]
fn distance_sq(a: &[f32; 3], b: &[f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_obj_cube() {
        let obj_data = b"
v -1 -1 -1
v  1 -1 -1
v  1  1 -1
v -1  1 -1
v -1 -1  1
v  1 -1  1
v  1  1  1
v -1  1  1
f 1 2 3 4
f 5 6 7 8
f 1 2 6 5
f 2 3 7 6
f 3 4 8 7
f 4 1 5 8
";
        let shapes = parse_obj(obj_data).unwrap();
        assert!(!shapes.is_empty());
        // A cube has 12 unique edges
        assert_eq!(shapes.len(), 12);
    }
}
