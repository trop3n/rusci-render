/// Network server configuration.
pub struct NetConfig {
    /// TCP port for Blender GPLA streaming.
    pub blender_port: u16,
    /// WebSocket port for JSON shape streaming.
    pub ws_port: u16,
    /// Bind address.
    pub bind_addr: String,
}

impl Default for NetConfig {
    fn default() -> Self {
        Self {
            blender_port: 51677,
            ws_port: 51678,
            bind_addr: "127.0.0.1".to_string(),
        }
    }
}
