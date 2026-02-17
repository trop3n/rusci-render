use glow::HasContext;

use crate::shaders;

/// Renders line segments as Gaussian beams using quad-per-segment geometry.
pub struct LineRenderer {
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    ibo: glow::Buffer,
    loc_sigma: glow::UniformLocation,
    loc_intensity: glow::UniformLocation,
    max_segments: usize,
}

impl LineRenderer {
    pub fn new(gl: &glow::Context, max_segments: usize) -> Self {
        let program = compile_program(gl, shaders::LINE_VERTEX, shaders::LINE_FRAGMENT);

        let loc_sigma = unsafe { gl.get_uniform_location(program, "u_sigma").expect("u_sigma") };
        let loc_intensity = unsafe { gl.get_uniform_location(program, "u_intensity").expect("u_intensity") };

        // Pre-allocate vertex buffer for max_segments * 4 vertices
        // Each vertex: pos(2) + other(2) + perp(1) + along(1) = 6 floats
        let vbo_size = max_segments * 4 * 6 * std::mem::size_of::<f32>();

        // Pre-allocate index buffer for max_segments * 6 indices
        let ibo_size = max_segments * 6 * std::mem::size_of::<u32>();

        unsafe {
            let vao = gl.create_vertex_array().expect("create vao");
            let vbo = gl.create_buffer().expect("create vbo");
            let ibo = gl.create_buffer().expect("create ibo");

            gl.bind_vertex_array(Some(vao));

            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_size(glow::ARRAY_BUFFER, vbo_size as i32, glow::DYNAMIC_DRAW);

            let stride = 6 * std::mem::size_of::<f32>() as i32;
            // a_pos: location 0
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, stride, 0);
            // a_other: location 1
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, stride, 8);
            // a_perp: location 2
            gl.enable_vertex_attrib_array(2);
            gl.vertex_attrib_pointer_f32(2, 1, glow::FLOAT, false, stride, 16);
            // a_along: location 3
            gl.enable_vertex_attrib_array(3);
            gl.vertex_attrib_pointer_f32(3, 1, glow::FLOAT, false, stride, 20);

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ibo));
            gl.buffer_data_size(glow::ELEMENT_ARRAY_BUFFER, ibo_size as i32, glow::DYNAMIC_DRAW);

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, None);

            Self {
                program,
                vao,
                vbo,
                ibo,
                loc_sigma,
                loc_intensity,
                max_segments,
            }
        }
    }

    /// Render line segments from x/y sample arrays into the currently bound FBO.
    /// Samples are in [-1, 1] and get mapped to [0, 1] UV space.
    pub fn render(&self, gl: &glow::Context, x_samples: &[f32], y_samples: &[f32], sigma: f32, intensity: f32) {
        let n = x_samples.len().min(y_samples.len());
        if n < 2 {
            return;
        }

        let num_segments = (n - 1).min(self.max_segments);

        // Build vertex data: 4 vertices per segment, 6 floats each
        let mut vertices = Vec::with_capacity(num_segments * 4 * 6);
        let mut indices = Vec::with_capacity(num_segments * 6);

        for i in 0..num_segments {
            // Map from [-1,1] to [0,1] UV space
            let ax = x_samples[i] * 0.5 + 0.5;
            let ay = (-y_samples[i]) * 0.5 + 0.5; // flip Y
            let bx = x_samples[i + 1] * 0.5 + 0.5;
            let by = (-y_samples[i + 1]) * 0.5 + 0.5;

            let base = (i * 4) as u32;

            // 4 corners of the quad: (along=0,perp=-1), (along=0,perp=+1), (along=1,perp=+1), (along=1,perp=-1)
            // vertex 0: start, perp=-1
            vertices.extend_from_slice(&[ax, ay, bx, by, -1.0, 0.0]);
            // vertex 1: start, perp=+1
            vertices.extend_from_slice(&[ax, ay, bx, by, 1.0, 0.0]);
            // vertex 2: end, perp=+1
            vertices.extend_from_slice(&[ax, ay, bx, by, 1.0, 1.0]);
            // vertex 3: end, perp=-1
            vertices.extend_from_slice(&[ax, ay, bx, by, -1.0, 1.0]);

            // Two triangles
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }

        unsafe {
            gl.use_program(Some(self.program));
            gl.uniform_1_f32(Some(&self.loc_sigma), sigma);
            gl.uniform_1_f32(Some(&self.loc_intensity), intensity);

            gl.bind_vertex_array(Some(self.vao));

            // Upload vertex data
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.buffer_sub_data_u8_slice(glow::ARRAY_BUFFER, 0, cast_slice_f32(&vertices));

            // Upload index data
            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ibo));
            gl.buffer_sub_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, 0, cast_slice_u32(&indices));

            // Additive blending
            gl.enable(glow::BLEND);
            gl.blend_func(glow::ONE, glow::ONE);

            gl.draw_elements(glow::TRIANGLES, (num_segments * 6) as i32, glow::UNSIGNED_INT, 0);

            gl.bind_vertex_array(None);
            gl.use_program(None);
        }
    }

    pub fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vao);
            gl.delete_buffer(self.vbo);
            gl.delete_buffer(self.ibo);
        }
    }
}

fn compile_program(gl: &glow::Context, vert_src: &str, frag_src: &str) -> glow::Program {
    unsafe {
        let program = gl.create_program().expect("create program");

        let vert = gl.create_shader(glow::VERTEX_SHADER).expect("create vertex shader");
        gl.shader_source(vert, vert_src);
        gl.compile_shader(vert);
        if !gl.get_shader_compile_status(vert) {
            let log = gl.get_shader_info_log(vert);
            panic!("Vertex shader compilation failed:\n{}", log);
        }

        let frag = gl.create_shader(glow::FRAGMENT_SHADER).expect("create fragment shader");
        gl.shader_source(frag, frag_src);
        gl.compile_shader(frag);
        if !gl.get_shader_compile_status(frag) {
            let log = gl.get_shader_info_log(frag);
            panic!("Fragment shader compilation failed:\n{}", log);
        }

        gl.attach_shader(program, vert);
        gl.attach_shader(program, frag);
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            let log = gl.get_program_info_log(program);
            panic!("Program linking failed:\n{}", log);
        }

        gl.delete_shader(vert);
        gl.delete_shader(frag);

        program
    }
}

fn cast_slice_f32(data: &[f32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * std::mem::size_of::<f32>())
    }
}

fn cast_slice_u32(data: &[u32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * std::mem::size_of::<u32>())
    }
}
