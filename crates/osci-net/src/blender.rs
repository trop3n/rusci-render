use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

use crate::config::NetConfig;
use crate::frame_channel::FrameSink;

const MAX_MESSAGE_SIZE: u32 = 64 * 1024 * 1024; // 64 MB

/// Start the Blender TCP server. Blocks until shutdown is signalled.
pub async fn start_blender_server(
    config: &NetConfig,
    sink: FrameSink,
    shutdown: Arc<AtomicBool>,
) -> Result<(), String> {
    let addr = format!("{}:{}", config.bind_addr, config.blender_port);
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("Blender TCP bind failed on {}: {}", addr, e))?;

    log::info!("Blender TCP server listening on {}", addr);

    loop {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }

        let accept = tokio::select! {
            result = listener.accept() => result,
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => continue,
        };

        match accept {
            Ok((stream, peer)) => {
                log::info!("Blender client connected: {}", peer);
                let sink_clone = FrameSink::new(sink.sender());
                let shutdown_clone = shutdown.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_blender_connection(stream, sink_clone, shutdown_clone).await {
                        log::warn!("Blender connection error: {}", e);
                    }
                    log::info!("Blender client disconnected: {}", peer);
                });
            }
            Err(e) => {
                log::warn!("Blender accept error: {}", e);
            }
        }
    }

    Ok(())
}

async fn handle_blender_connection(
    mut stream: tokio::net::TcpStream,
    sink: FrameSink,
    shutdown: Arc<AtomicBool>,
) -> Result<(), String> {
    loop {
        if shutdown.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Read 4-byte LE u32 length prefix
        let len = match stream.read_u32_le().await {
            Ok(len) => len,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(format!("Read length error: {}", e)),
        };

        if len > MAX_MESSAGE_SIZE {
            return Err(format!("Message too large: {} bytes", len));
        }

        // Read the payload
        let mut buf = vec![0u8; len as usize];
        stream.read_exact(&mut buf).await.map_err(|e| format!("Read payload error: {}", e))?;

        // Parse GPLA
        match osci_parsers::gpla::parse_gpla(&buf) {
            Ok(gpla_frames) => {
                for frame in gpla_frames.frames {
                    sink.send(frame);
                }
            }
            Err(e) => {
                log::warn!("GPLA parse error: {}", e);
            }
        }
    }
}
