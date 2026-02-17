use glow::HasContext;

use crate::quad::FullscreenQuad;
use crate::settings::VisualiserSettings;
use crate::shaders;

/// Final compositing pass: combines persisted lines + bloom, applies tone mapping and color.
pub struct Compositor {
    program: glow::Program,
    loc_persisted: glow::UniformLocation,
    loc_tight_blur: glow::UniformLocation,
    loc_wide_blur: glow::UniformLocation,
    loc_color: glow::UniformLocation,
    loc_exposure: glow::UniformLocation,
    loc_glow_amount: glow::UniformLocation,
    loc_scatter_amount: glow::UniformLocation,
    loc_overexposure: glow::UniformLocation,
    loc_saturation: glow::UniformLocation,
    loc_ambient: glow::UniformLocation,
    loc_noise: glow::UniformLocation,
    loc_time: glow::UniformLocation,
    frame_count: u32,
}

impl Compositor {
    pub fn new(gl: &glow::Context) -> Self {
        let program = compile_fullscreen_program(gl, shaders::COMPOSITE_FRAGMENT);

        unsafe {
            let loc = |name: &str| gl.get_uniform_location(program, name).expect(name);
            Self {
                program,
                loc_persisted: loc("u_persisted"),
                loc_tight_blur: loc("u_tight_blur"),
                loc_wide_blur: loc("u_wide_blur"),
                loc_color: loc("u_color"),
                loc_exposure: loc("u_exposure"),
                loc_glow_amount: loc("u_glow_amount"),
                loc_scatter_amount: loc("u_scatter_amount"),
                loc_overexposure: loc("u_overexposure"),
                loc_saturation: loc("u_saturation"),
                loc_ambient: loc("u_ambient"),
                loc_noise: loc("u_noise"),
                loc_time: loc("u_time"),
                frame_count: 0,
            }
        }
    }

    /// Render the final composited image to the currently bound FBO.
    pub fn render(
        &mut self,
        gl: &glow::Context,
        persisted_tex: glow::Texture,
        tight_tex: glow::Texture,
        wide_tex: glow::Texture,
        settings: &VisualiserSettings,
        quad: &FullscreenQuad,
    ) {
        self.frame_count = self.frame_count.wrapping_add(1);

        unsafe {
            gl.use_program(Some(self.program));
            gl.disable(glow::BLEND);

            // Bind textures
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(persisted_tex));
            gl.uniform_1_i32(Some(&self.loc_persisted), 0);

            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, Some(tight_tex));
            gl.uniform_1_i32(Some(&self.loc_tight_blur), 1);

            gl.active_texture(glow::TEXTURE2);
            gl.bind_texture(glow::TEXTURE_2D, Some(wide_tex));
            gl.uniform_1_i32(Some(&self.loc_wide_blur), 2);

            // Set uniforms
            gl.uniform_3_f32(Some(&self.loc_color), settings.color[0], settings.color[1], settings.color[2]);
            gl.uniform_1_f32(Some(&self.loc_exposure), settings.exposure);
            gl.uniform_1_f32(Some(&self.loc_glow_amount), settings.glow_amount);
            gl.uniform_1_f32(Some(&self.loc_scatter_amount), settings.scatter_amount);
            gl.uniform_1_f32(Some(&self.loc_overexposure), settings.overexposure);
            gl.uniform_1_f32(Some(&self.loc_saturation), settings.saturation);
            gl.uniform_1_f32(Some(&self.loc_ambient), settings.ambient);
            gl.uniform_1_f32(Some(&self.loc_noise), settings.noise);
            gl.uniform_1_f32(Some(&self.loc_time), self.frame_count as f32 * 0.0167);

            quad.draw(gl);

            gl.active_texture(glow::TEXTURE0);
            gl.use_program(None);
        }
    }

    pub fn destroy(&self, gl: &glow::Context) {
        unsafe { gl.delete_program(self.program); }
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
