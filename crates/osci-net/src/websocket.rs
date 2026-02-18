use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use futures_util::StreamExt;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

use crate::config::NetConfig;
use crate::frame_channel::FrameSink;

/// Start the WebSocket server. Blocks until shutdown is signalled.
pub async fn start_ws_server(
    config: &NetConfig,
    sink: FrameSink,
    shutdown: Arc<AtomicBool>,
) -> Result<(), String> {
    let addr = format!("{}:{}", config.bind_addr, config.ws_port);
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("WebSocket bind failed on {}: {}", addr, e))?;

    log::info!("WebSocket server listening on {}", addr);

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
                log::info!("WebSocket client connected: {}", peer);
                let sink_clone = FrameSink::new(sink.sender());
                let shutdown_clone = shutdown.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_ws_connection(stream, sink_clone, shutdown_clone).await {
                        log::warn!("WebSocket connection error: {}", e);
                    }
                    log::info!("WebSocket client disconnected: {}", peer);
                });
            }
            Err(e) => {
                log::warn!("WebSocket accept error: {}", e);
            }
        }
    }

    Ok(())
}

async fn handle_ws_connection(
    stream: tokio::net::TcpStream,
    sink: FrameSink,
    shutdown: Arc<AtomicBool>,
) -> Result<(), String> {
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .map_err(|e| format!("WebSocket handshake failed: {}", e))?;

    let (_, mut read) = ws_stream.split();

    while let Some(msg_result) = read.next().await {
        if shutdown.load(Ordering::Relaxed) {
            break;
        }

        let msg = match msg_result {
            Ok(m) => m,
            Err(e) => {
                log::warn!("WebSocket read error: {}", e);
                break;
            }
        };

        match msg {
            Message::Text(text) => {
                handle_json_message(&text, &sink);
            }
            Message::Binary(data) => {
                // Binary messages treated as raw GPLA data
                match osci_parsers::gpla::parse_gpla(&data) {
                    Ok(gpla_frames) => {
                        for frame in gpla_frames.frames {
                            sink.send(frame);
                        }
                    }
                    Err(e) => {
                        log::warn!("WebSocket GPLA parse error: {}", e);
                    }
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }

    Ok(())
}

#[derive(serde::Deserialize)]
struct WsMessage {
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)]
    lines: Vec<WsLine>,
}

#[derive(serde::Deserialize)]
struct WsLine {
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
}

fn handle_json_message(text: &str, sink: &FrameSink) {
    let msg: WsMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(e) => {
            log::warn!("WebSocket JSON parse error: {}", e);
            return;
        }
    };

    match msg.msg_type.as_str() {
        "shapes" => {
            let shapes: Vec<Box<dyn osci_core::shape::Shape>> = msg
                .lines
                .iter()
                .map(|l| {
                    Box::new(osci_core::shape::Line::new_2d(l.x0, l.y0, l.x1, l.y1))
                        as Box<dyn osci_core::shape::Shape>
                })
                .collect();
            sink.send(shapes);
        }
        "ping" => {
            // No-op, keep connection alive
        }
        other => {
            log::warn!("Unknown WebSocket message type: {}", other);
        }
    }
}
