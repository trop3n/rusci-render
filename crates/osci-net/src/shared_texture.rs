/// Trait for shared texture output (Spout on Windows, Syphon on macOS).
pub trait SharedTexture {
    fn init(&mut self, gl: &glow::Context, name: &str) -> Result<(), String>;
    fn send_texture(
        &mut self,
        gl: &glow::Context,
        texture: glow::Texture,
        width: u32,
        height: u32,
    ) -> Result<(), String>;
    fn shutdown(&mut self, gl: &glow::Context);
    fn is_available() -> bool
    where
        Self: Sized;
}

/// No-op stub for unsupported platforms.
pub struct NoOpSharedTexture;

impl SharedTexture for NoOpSharedTexture {
    fn init(&mut self, _gl: &glow::Context, _name: &str) -> Result<(), String> {
        Ok(())
    }

    fn send_texture(
        &mut self,
        _gl: &glow::Context,
        _texture: glow::Texture,
        _width: u32,
        _height: u32,
    ) -> Result<(), String> {
        Ok(())
    }

    fn shutdown(&mut self, _gl: &glow::Context) {}

    fn is_available() -> bool {
        false
    }
}

/// Create the appropriate shared texture implementation for the current platform.
pub fn create_shared_texture() -> Box<dyn SharedTexture + Send> {
    Box::new(NoOpSharedTexture)
}
