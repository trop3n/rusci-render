use glow::HasContext;

use crate::bloom::BloomPass;
use crate::compositor::Compositor;
use crate::fbo::RenderTarget;
use crate::line_renderer::LineRenderer;
use crate::persistence::PersistencePass;
use crate::quad::FullscreenQuad;
use crate::settings::VisualiserSettings;

const LINE_FBO_SIZE: u32 = 1024;
const MAX_SEGMENTS: usize = 2048;

/// Saved OpenGL state so we can restore egui's GL context after custom rendering.
struct SavedGlState {
    framebuffer: Option<glow::Framebuffer>,
    viewport: [i32; 4],
    blend_enabled: bool,
    blend_src_rgb: i32,
    blend_dst_rgb: i32,
    blend_src_alpha: i32,
    blend_dst_alpha: i32,
    blend_eq_rgb: i32,
    blend_eq_alpha: i32,
    program: Option<glow::Program>,
    vao: Option<glow::VertexArray>,
    scissor_enabled: bool,
    active_texture: i32,
    bound_textures: [Option<glow::Texture>; 4],
}

impl SavedGlState {
    unsafe fn save(gl: &glow::Context) -> Self {
        let framebuffer = {
            let v = gl.get_parameter_i32(glow::FRAMEBUFFER_BINDING);
            if v == 0 { None } else { Some(glow::NativeFramebuffer(std::num::NonZeroU32::new(v as u32).unwrap())) }
        };

        let mut viewport = [0i32; 4];
        gl.get_parameter_i32_slice(glow::VIEWPORT, &mut viewport);

        let blend_enabled = gl.is_enabled(glow::BLEND);
        let blend_src_rgb = gl.get_parameter_i32(glow::BLEND_SRC_RGB);
        let blend_dst_rgb = gl.get_parameter_i32(glow::BLEND_DST_RGB);
        let blend_src_alpha = gl.get_parameter_i32(glow::BLEND_SRC_ALPHA);
        let blend_dst_alpha = gl.get_parameter_i32(glow::BLEND_DST_ALPHA);
        let blend_eq_rgb = gl.get_parameter_i32(glow::BLEND_EQUATION_RGB);
        let blend_eq_alpha = gl.get_parameter_i32(glow::BLEND_EQUATION_ALPHA);

        let program_id = gl.get_parameter_i32(glow::CURRENT_PROGRAM);
        let program = if program_id == 0 {
            None
        } else {
            Some(glow::NativeProgram(std::num::NonZeroU32::new(program_id as u32).unwrap()))
        };

        let vao_id = gl.get_parameter_i32(glow::VERTEX_ARRAY_BINDING);
        let vao = if vao_id == 0 {
            None
        } else {
            Some(glow::NativeVertexArray(std::num::NonZeroU32::new(vao_id as u32).unwrap()))
        };

        let scissor_enabled = gl.is_enabled(glow::SCISSOR_TEST);
        let active_texture = gl.get_parameter_i32(glow::ACTIVE_TEXTURE);

        let mut bound_textures = [None; 4];
        for i in 0..4 {
            gl.active_texture(glow::TEXTURE0 + i as u32);
            let tex_id = gl.get_parameter_i32(glow::TEXTURE_BINDING_2D);
            if tex_id != 0 {
                bound_textures[i] = Some(glow::NativeTexture(std::num::NonZeroU32::new(tex_id as u32).unwrap()));
            }
        }
        gl.active_texture(active_texture as u32);

        Self {
            framebuffer,
            viewport,
            blend_enabled,
            blend_src_rgb,
            blend_dst_rgb,
            blend_src_alpha,
            blend_dst_alpha,
            blend_eq_rgb,
            blend_eq_alpha,
            program,
            vao,
            scissor_enabled,
            active_texture,
            bound_textures,
        }
    }

    unsafe fn restore(&self, gl: &glow::Context) {
        gl.bind_framebuffer(glow::FRAMEBUFFER, self.framebuffer);
        gl.viewport(self.viewport[0], self.viewport[1], self.viewport[2], self.viewport[3]);

        if self.blend_enabled {
            gl.enable(glow::BLEND);
        } else {
            gl.disable(glow::BLEND);
        }
        gl.blend_func_separate(
            self.blend_src_rgb as u32,
            self.blend_dst_rgb as u32,
            self.blend_src_alpha as u32,
            self.blend_dst_alpha as u32,
        );
        gl.blend_equation_separate(self.blend_eq_rgb as u32, self.blend_eq_alpha as u32);

        gl.use_program(self.program);
        gl.bind_vertex_array(self.vao);

        if self.scissor_enabled {
            gl.enable(glow::SCISSOR_TEST);
        } else {
            gl.disable(glow::SCISSOR_TEST);
        }

        for i in 0..4 {
            gl.active_texture(glow::TEXTURE0 + i as u32);
            gl.bind_texture(glow::TEXTURE_2D, self.bound_textures[i]);
        }
        gl.active_texture(self.active_texture as u32);
    }
}

/// Orchestrates the full GPU oscilloscope rendering pipeline.
pub struct OsciRenderer {
    line_fbo: RenderTarget,
    line_renderer: LineRenderer,
    bloom: BloomPass,
    persistence: PersistencePass,
    compositor: Compositor,
    quad: FullscreenQuad,
}

impl OsciRenderer {
    /// Create a new renderer. Must be called with a valid GL context.
    pub fn new(gl: &glow::Context) -> Self {
        Self {
            line_fbo: RenderTarget::new(gl, LINE_FBO_SIZE, LINE_FBO_SIZE),
            line_renderer: LineRenderer::new(gl, MAX_SEGMENTS),
            bloom: BloomPass::new(gl),
            persistence: PersistencePass::new(gl),
            compositor: Compositor::new(gl),
            quad: FullscreenQuad::new(gl),
        }
    }

    /// Render the oscilloscope visualization.
    ///
    /// `viewport` is [x, y, width, height] in physical pixels for the final output.
    pub fn render(
        &mut self,
        gl: &glow::Context,
        x_samples: &[f32],
        y_samples: &[f32],
        settings: &VisualiserSettings,
        viewport: [i32; 4],
    ) {
        unsafe {
            // 1. Save egui's GL state
            let saved = SavedGlState::save(gl);

            gl.disable(glow::SCISSOR_TEST);

            // 2. Render lines into line FBO (1024x1024, additive blend)
            self.line_fbo.bind(gl);
            gl.clear_color(0.0, 0.0, 0.0, 0.0);
            gl.clear(glow::COLOR_BUFFER_BIT);
            self.line_renderer.render(gl, x_samples, y_samples, settings.focus, settings.intensity);

            // 3. Persistence: blend with previous frame
            let persisted_tex = self.persistence.render(
                gl,
                self.line_fbo.texture,
                settings.persistence,
                settings.afterglow,
                &settings.afterglow_color,
                &self.quad,
            );

            // 4. Bloom: tight + wide blur
            let (tight_tex, wide_tex) = self.bloom.render(gl, persisted_tex, &self.quad);

            // 5. Restore egui's FBO, set viewport to target rect
            gl.bind_framebuffer(glow::FRAMEBUFFER, saved.framebuffer);
            gl.viewport(viewport[0], viewport[1], viewport[2], viewport[3]);

            // 6. Composite final image
            self.compositor.render(gl, persisted_tex, tight_tex, wide_tex, settings, &self.quad);

            // 7. Restore all GL state
            saved.restore(gl);
        }
    }

    /// Read pixels from the currently bound FBO. Returns RGBA8 data, flipped vertically.
    pub fn capture_frame(&self, gl: &glow::Context, width: u32, height: u32) -> Vec<u8> {
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        unsafe {
            gl.read_pixels(
                0,
                0,
                width as i32,
                height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
        }
        // Flip vertically (OpenGL origin is bottom-left)
        let row_bytes = (width * 4) as usize;
        let mut flipped = vec![0u8; pixels.len()];
        for y in 0..height as usize {
            let src_row = y * row_bytes;
            let dst_row = (height as usize - 1 - y) * row_bytes;
            flipped[dst_row..dst_row + row_bytes].copy_from_slice(&pixels[src_row..src_row + row_bytes]);
        }
        flipped
    }

    pub fn destroy(&self, gl: &glow::Context) {
        self.line_fbo.destroy(gl);
        self.line_renderer.destroy(gl);
        self.bloom.destroy(gl);
        self.persistence.destroy(gl);
        self.compositor.destroy(gl);
        self.quad.destroy(gl);
    }
}
