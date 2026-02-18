use glow::HasContext;
use std::time::Instant;

use crate::fbo::RenderTarget;
use crate::quad::FullscreenQuad;
use crate::shaders;

/// Phosphor persistence via ping-pong FBOs with exponential decay.
pub struct PersistencePass {
    program: glow::Program,
    targets: [RenderTarget; 2],
    current_idx: usize,
    last_frame: Instant,
    loc_current: glow::UniformLocation,
    loc_previous: glow::UniformLocation,
    loc_fade: glow::UniformLocation,
    loc_afterglow_color: glow::UniformLocation,
    loc_afterglow: glow::UniformLocation,
}

impl PersistencePass {
    pub fn new(gl: &glow::Context) -> Self {
        let program = compile_fullscreen_program(gl, shaders::PERSISTENCE_FRAGMENT);

        let loc_current = unsafe { gl.get_uniform_location(program, "u_current").expect("u_current") };
        let loc_previous = unsafe { gl.get_uniform_location(program, "u_previous").expect("u_previous") };
        let loc_fade = unsafe { gl.get_uniform_location(program, "u_fade").expect("u_fade") };
        let loc_afterglow_color = unsafe { gl.get_uniform_location(program, "u_afterglow_color").expect("u_afterglow_color") };
        let loc_afterglow = unsafe { gl.get_uniform_location(program, "u_afterglow").expect("u_afterglow") };

        Self {
            program,
            targets: [
                RenderTarget::new(gl, 1024, 1024),
                RenderTarget::new(gl, 1024, 1024),
            ],
            current_idx: 0,
            last_frame: Instant::now(),
            loc_current,
            loc_previous,
            loc_fade,
            loc_afterglow_color,
            loc_afterglow,
        }
    }

    /// Blend current line texture with previous frame.
    /// Returns the persisted texture handle.
    pub fn render(
        &mut self,
        gl: &glow::Context,
        line_texture: glow::Texture,
        persistence: f32,
        afterglow: f32,
        afterglow_color: &[f32; 3],
        quad: &FullscreenQuad,
    ) -> glow::Texture {
        let now = Instant::now();
        let dt = now.duration_since(self.last_frame).as_secs_f32();
        self.last_frame = now;

        // Calculate fade factor: exponential decay scaled by frame time
        // At persistence=0.5, about 40% retained per frame at 60fps
        let fps_ref = 60.0;
        let fade = (0.5f32).powf(1.0 - persistence) * 0.4 * (fps_ref * dt);
        let fade = fade.clamp(0.0, 0.99);

        let prev_idx = self.current_idx;
        let next_idx = 1 - self.current_idx;

        unsafe {
            gl.use_program(Some(self.program));
            gl.disable(glow::BLEND);

            // Bind output
            self.targets[next_idx].bind(gl);
            gl.clear(glow::COLOR_BUFFER_BIT);

            // Bind current line texture to unit 0
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(line_texture));
            gl.uniform_1_i32(Some(&self.loc_current), 0);

            // Bind previous frame to unit 1
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.targets[prev_idx].texture));
            gl.uniform_1_i32(Some(&self.loc_previous), 1);

            gl.uniform_1_f32(Some(&self.loc_fade), fade);
            gl.uniform_3_f32(
                Some(&self.loc_afterglow_color),
                afterglow_color[0],
                afterglow_color[1],
                afterglow_color[2],
            );
            gl.uniform_1_f32(Some(&self.loc_afterglow), afterglow);

            quad.draw(gl);

            gl.active_texture(glow::TEXTURE0);
            gl.use_program(None);
        }

        self.current_idx = next_idx;
        self.targets[next_idx].texture
    }

    pub fn destroy(&self, gl: &glow::Context) {
        unsafe { gl.delete_program(self.program); }
        self.targets[0].destroy(gl);
        self.targets[1].destroy(gl);
    }
}

fn compile_fullscreen_program(gl: &glow::Context, frag_src: &str) -> glow::Program {
    unsafe {
        let program = gl.create_program().expect("create program");

        let vert = gl.create_shader(glow::VERTEX_SHADER).expect("create vertex shader");
        gl.shader_source(vert, shaders::FULLSCREEN_VERTEX);
        gl.compile_shader(vert);
        if !gl.get_shader_compile_status(vert) {
            panic!("Vertex shader failed:\n{}", gl.get_shader_info_log(vert));
        }

        let frag = gl.create_shader(glow::FRAGMENT_SHADER).expect("create fragment shader");
        gl.shader_source(frag, frag_src);
        gl.compile_shader(frag);
        if !gl.get_shader_compile_status(frag) {
            panic!("Fragment shader failed:\n{}", gl.get_shader_info_log(frag));
        }

        gl.attach_shader(program, vert);
        gl.attach_shader(program, frag);
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            panic!("Program linking failed:\n{}", gl.get_program_info_log(program));
        }

        gl.delete_shader(vert);
        gl.delete_shader(frag);
        program
    }
}
