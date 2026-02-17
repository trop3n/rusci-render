use glow::HasContext;

use crate::fbo::RenderTarget;
use crate::quad::FullscreenQuad;
use crate::shaders;

/// Separable Gaussian blur bloom pass.
///
/// Produces two bloom textures:
/// - Tight: 512x512, 17-tap blur
/// - Wide: 128x128, 65-tap blur
pub struct BloomPass {
    program: glow::Program,
    // Tight bloom: 512x512
    tight_a: RenderTarget, // horizontal pass
    tight_b: RenderTarget, // vertical pass (final tight result)
    // Wide bloom: 128x128
    wide_a: RenderTarget,  // horizontal pass
    wide_b: RenderTarget,  // vertical pass (final wide result)
    loc_texture: glow::UniformLocation,
    loc_direction: glow::UniformLocation,
    loc_tap_count: glow::UniformLocation,
}

impl BloomPass {
    pub fn new(gl: &glow::Context) -> Self {
        let program = compile_fullscreen_program(gl, shaders::BLUR_FRAGMENT);

        let loc_texture = unsafe { gl.get_uniform_location(program, "u_texture").expect("u_texture") };
        let loc_direction = unsafe { gl.get_uniform_location(program, "u_direction").expect("u_direction") };
        let loc_tap_count = unsafe { gl.get_uniform_location(program, "u_tap_count").expect("u_tap_count") };

        Self {
            program,
            tight_a: RenderTarget::new(gl, 512, 512),
            tight_b: RenderTarget::new(gl, 512, 512),
            wide_a: RenderTarget::new(gl, 128, 128),
            wide_b: RenderTarget::new(gl, 128, 128),
            loc_texture,
            loc_direction,
            loc_tap_count,
        }
    }

    /// Run bloom passes on the given source texture.
    /// Returns (tight_texture, wide_texture) handles.
    pub fn render(
        &self,
        gl: &glow::Context,
        source_texture: glow::Texture,
        quad: &FullscreenQuad,
    ) -> (glow::Texture, glow::Texture) {
        unsafe {
            gl.use_program(Some(self.program));
            gl.active_texture(glow::TEXTURE0);
            gl.uniform_1_i32(Some(&self.loc_texture), 0);
            gl.disable(glow::BLEND);

            // ── Tight bloom (512x512, 8-tap half = 17-tap total) ──
            // Horizontal pass: source -> tight_a
            self.tight_a.bind(gl);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.bind_texture(glow::TEXTURE_2D, Some(source_texture));
            gl.uniform_2_f32(Some(&self.loc_direction), 1.0 / 512.0, 0.0);
            gl.uniform_1_i32(Some(&self.loc_tap_count), 8);
            quad.draw(gl);

            // Vertical pass: tight_a -> tight_b
            self.tight_b.bind(gl);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.tight_a.texture));
            gl.uniform_2_f32(Some(&self.loc_direction), 0.0, 1.0 / 512.0);
            gl.uniform_1_i32(Some(&self.loc_tap_count), 8);
            quad.draw(gl);

            // ── Wide bloom (128x128, 32-tap half = 65-tap total) ──
            // Horizontal pass: source -> wide_a
            self.wide_a.bind(gl);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.bind_texture(glow::TEXTURE_2D, Some(source_texture));
            gl.uniform_2_f32(Some(&self.loc_direction), 1.0 / 128.0, 0.0);
            gl.uniform_1_i32(Some(&self.loc_tap_count), 32);
            quad.draw(gl);

            // Vertical pass: wide_a -> wide_b
            self.wide_b.bind(gl);
            gl.clear(glow::COLOR_BUFFER_BIT);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.wide_a.texture));
            gl.uniform_2_f32(Some(&self.loc_direction), 0.0, 1.0 / 128.0);
            gl.uniform_1_i32(Some(&self.loc_tap_count), 32);
            quad.draw(gl);

            gl.use_program(None);
        }

        (self.tight_b.texture, self.wide_b.texture)
    }

    pub fn destroy(&self, gl: &glow::Context) {
        unsafe { gl.delete_program(self.program); }
        self.tight_a.destroy(gl);
        self.tight_b.destroy(gl);
        self.wide_a.destroy(gl);
        self.wide_b.destroy(gl);
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
