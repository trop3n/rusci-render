//! Live line art from osci-render's Blender add-on.
//!
//! Same protocol as osci-render's `ObjectServer`: the add-on connects over TCP
//! and sends newline-terminated text messages:
//!
//! - a base64-encoded binary GPLA file (starts with `R1BMQSAg`, base64 for
//!   `"GPLA  "`), sent on connect and after every frame change or scene edit;
//!   its first frame is drawn
//! - the older single-frame JSON format, `{"objects": [...], "focalLength": f}`
//! - `CLOSE`, sent when the add-on disconnects
//!
//! Empty lines are ignored. A message over 10 MB or containing a NUL byte
//! drops the connection; a message that fails to parse is logged and skipped.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use base64::Engine;
use osci_core::shape::Shape;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::net::TcpListener;

use crate::config::NetConfig;
use crate::frame_channel::FrameSink;

/// Longest message accepted, as in osci-render.
const MAX_MESSAGE_BYTES: usize = 10 * 1024 * 1024;

/// How often an idle connection checks for shutdown.
const POLL_INTERVAL: Duration = Duration::from_millis(200);

/// Base64 encoding of `"GPLA  "`, the start of every binary GPLA file.
const BASE64_GPLA_PREFIX: &[u8] = b"R1BMQSAg";

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
                    if let Err(e) =
                        handle_blender_connection(stream, sink_clone, shutdown_clone).await
                    {
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

/// Read newline-terminated messages until the client sends `CLOSE`,
/// disconnects, or shutdown is signalled.
async fn handle_blender_connection<S: AsyncRead + Unpin>(
    mut stream: S,
    sink: FrameSink,
    shutdown: Arc<AtomicBool>,
) -> Result<(), String> {
    let mut message = Vec::new();
    let mut chunk = vec![0u8; 64 * 1024];

    loop {
        if shutdown.load(Ordering::Relaxed) {
            return Ok(());
        }

        let n = match tokio::time::timeout(POLL_INTERVAL, stream.read(&mut chunk)).await {
            Err(_) => continue, // idle: check shutdown again
            Ok(Ok(0)) => return Ok(()),
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(format!("Read error: {}", e)),
        };

        let mut rest = &chunk[..n];
        while let Some(end) = rest.iter().position(|&b| b == b'\n') {
            append(&mut message, &rest[..end])?;
            match decode_message(&message) {
                Message::Close => return Ok(()),
                Message::Frame(frame) => {
                    sink.send(frame);
                }
                Message::Skip(None) => {}
                Message::Skip(Some(e)) => log::warn!("Blender message skipped: {}", e),
            }
            message.clear();
            rest = &rest[end + 1..];
        }
        append(&mut message, rest)?;
    }
}

/// Add part of a message, enforcing the size limit and rejecting NUL bytes.
fn append(message: &mut Vec<u8>, part: &[u8]) -> Result<(), String> {
    if part.contains(&0) {
        return Err("message contains a NUL byte".to_string());
    }
    if message.len() + part.len() > MAX_MESSAGE_BYTES {
        return Err(format!(
            "message is over the {MAX_MESSAGE_BYTES}-byte limit"
        ));
    }
    message.extend_from_slice(part);
    Ok(())
}

enum Message {
    Close,
    Frame(Vec<Box<dyn Shape>>),
    /// Nothing to draw: an empty line, or a message that failed to parse.
    Skip(Option<String>),
}

/// Decode one message (without its newline).
fn decode_message(line: &[u8]) -> Message {
    let text = line.trim_ascii_end();
    if text == b"CLOSE" {
        return Message::Close;
    }
    if text.is_empty() {
        return Message::Skip(None);
    }

    if text.starts_with(BASE64_GPLA_PREFIX) {
        let binary = match base64::engine::general_purpose::STANDARD.decode(text) {
            Ok(binary) => binary,
            Err(e) => return Message::Skip(Some(format!("invalid base64: {e}"))),
        };
        return match osci_parsers::gpla::parse_gpla(&binary) {
            Ok(gpla) => match gpla.frames.into_iter().next() {
                Some(frame) => Message::Frame(frame),
                None => Message::Skip(Some("GPLA data has no frames".to_string())),
            },
            Err(e) => Message::Skip(Some(e)),
        };
    }

    match osci_parsers::gpla::parse_json_frame(text) {
        Ok(frame) => Message::Frame(frame),
        Err(e) => Message::Skip(Some(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam::channel::{bounded, Receiver};
    use tokio::io::AsyncWriteExt;

    /// A one-frame binary GPLA file as the Blender add-on builds it
    /// (`get_gpla_file`): an "L" 5 units in front of a 50 mm camera.
    fn addon_gpla() -> Vec<u8> {
        let mut out = b"GPLA    ".to_vec();
        for v in [2u64, 0, 0] {
            out.extend(v.to_le_bytes());
        }
        out.extend(b"FILE    fCount  ");
        out.extend(1u64.to_le_bytes());
        out.extend(b"fRate   ");
        out.extend(24u64.to_le_bytes());
        out.extend(b"DONE    FRAME   focalLen");
        out.extend((-0.05f64 * 50.0).to_le_bytes());
        out.extend(b"OBJECTS OBJECT  MATRIX  ");
        for v in [
            1.0f64, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ] {
            out.extend(v.to_le_bytes());
        }
        out.extend(b"DONE    STROKES STROKE  vertexCt");
        out.extend(3u64.to_le_bytes());
        out.extend(b"VERTICES");
        for v in [0.0f64, 1.0, -5.0, 0.0, 0.0, -5.0, 0.5, 0.0, -5.0] {
            out.extend(v.to_le_bytes());
        }
        out.extend(b"DONE    DONE    DONE    DONE    DONE    DONE    END GPLA");
        out
    }

    /// One line exactly as the add-on sends it: `base64.b64encode(...) + b"\n"`.
    fn addon_line() -> Vec<u8> {
        let mut line = base64::engine::general_purpose::STANDARD
            .encode(addon_gpla())
            .into_bytes();
        line.push(b'\n');
        line
    }

    /// Endpoints of every line in a frame, rounded to 1e-4.
    fn endpoints(frame: &[Box<dyn Shape>]) -> Vec<[f32; 4]> {
        let r = |v: f32| (v * 1e4).round() / 1e4;
        frame
            .iter()
            .map(|s| {
                let (a, b) = (s.next_vector(0.0), s.next_vector(1.0));
                [r(a.x), r(a.y), r(b.x), r(b.y)]
            })
            .collect()
    }

    const L_SHAPE: [[f32; 4]; 2] = [[0.0, 0.5, 0.0, 0.0], [0.0, 0.0, 0.25, 0.0]];

    /// Feed `input` to a connection handler in `chunk`-sized writes. Returns
    /// the handler's result and every frame it sent.
    async fn run(input: &[u8], chunk: usize) -> (Result<(), String>, Vec<Vec<Box<dyn Shape>>>) {
        let (tx, rx) = bounded(64);
        let (mut client, server) = tokio::io::duplex(4096);
        let handler = tokio::spawn(handle_blender_connection(
            server,
            FrameSink::new(tx),
            Arc::new(AtomicBool::new(false)),
        ));
        for part in input.chunks(chunk) {
            // The handler may already have closed the connection.
            if client.write_all(part).await.is_err() {
                break;
            }
        }
        drop(client);
        let result = handler.await.unwrap();
        (result, drain(&rx))
    }

    fn drain(rx: &Receiver<Vec<Box<dyn Shape>>>) -> Vec<Vec<Box<dyn Shape>>> {
        rx.try_iter().collect()
    }

    #[tokio::test]
    async fn addon_frames_arrive_upright() {
        let mut input = addon_line();
        input.extend(addon_line());
        input.extend(b"CLOSE\n");
        let (result, frames) = run(&input, 1 << 20).await;
        result.unwrap();
        assert_eq!(frames.len(), 2);
        for frame in &frames {
            assert_eq!(endpoints(frame), L_SHAPE);
        }
    }

    #[tokio::test]
    async fn messages_split_across_reads() {
        let mut input = addon_line();
        input.extend(addon_line());
        let (result, frames) = run(&input, 7).await;
        result.unwrap();
        assert_eq!(frames.len(), 2);
    }

    #[tokio::test]
    async fn close_ends_the_connection() {
        let mut input = b"CLOSE\n".to_vec();
        input.extend(addon_line());
        let (result, frames) = run(&input, 1 << 20).await;
        result.unwrap();
        assert!(frames.is_empty(), "frame after CLOSE was drawn");
    }

    #[tokio::test]
    async fn crlf_and_blank_lines_are_fine() {
        let mut input = b"\n\r\n".to_vec();
        let mut line = addon_line();
        line.insert(line.len() - 1, b'\r');
        input.extend(line);
        input.extend(b"CLOSE\r\n");
        let (result, frames) = run(&input, 1 << 20).await;
        result.unwrap();
        assert_eq!(frames.len(), 1);
    }

    #[tokio::test]
    async fn legacy_json_frame() {
        let input = br#"{"objects":[{"name":"Line Art","vertices":[[{"x":0,"y":1,"z":-5},{"x":0,"y":0,"z":-5},{"x":0.5,"y":0,"z":-5}]],"matrix":[1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1]}],"focalLength":-2.5}
"#;
        let (result, frames) = run(input, 1 << 20).await;
        result.unwrap();
        assert_eq!(frames.len(), 1);
        assert_eq!(endpoints(&frames[0]), L_SHAPE);
    }

    #[tokio::test]
    async fn bad_messages_are_skipped() {
        let mut input = b"not json\nR1BMQSAg!!!not base64\nR1BMQSAgAAAA\n".to_vec();
        input.extend(addon_line());
        let (result, frames) = run(&input, 1 << 20).await;
        result.unwrap();
        assert_eq!(frames.len(), 1, "connection should survive bad messages");
    }

    #[tokio::test]
    async fn nul_byte_drops_the_connection() {
        let mut input = b"abc\0def\n".to_vec();
        input.extend(addon_line());
        let (result, frames) = run(&input, 1 << 20).await;
        assert!(result.unwrap_err().contains("NUL"));
        assert!(frames.is_empty());
    }

    #[tokio::test]
    async fn oversized_message_drops_the_connection() {
        let input = vec![b'a'; MAX_MESSAGE_BYTES + 1];
        let (result, _) = run(&input, 1 << 20).await;
        assert!(result.unwrap_err().contains("limit"));
    }

    #[tokio::test]
    async fn message_at_the_limit_is_accepted() {
        // Trailing spaces pad a real frame to exactly the limit.
        let mut input = addon_line();
        input.pop();
        input.resize(MAX_MESSAGE_BYTES, b' ');
        input.push(b'\n');
        let (result, frames) = run(&input, 1 << 20).await;
        result.unwrap();
        assert_eq!(frames.len(), 1);
    }

    #[tokio::test]
    async fn idle_connection_stops_on_shutdown() {
        let (tx, _rx) = bounded(1);
        let (_client, server) = tokio::io::duplex(64);
        let shutdown = Arc::new(AtomicBool::new(false));
        let handler = tokio::spawn(handle_blender_connection(
            server,
            FrameSink::new(tx),
            shutdown.clone(),
        ));
        tokio::time::sleep(Duration::from_millis(50)).await;
        shutdown.store(true, Ordering::Relaxed);
        let result = tokio::time::timeout(Duration::from_secs(2), handler).await;
        result
            .expect("handler kept running after shutdown")
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn server_accepts_addon_over_tcp() {
        // Find a free port, then run the real server on it.
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let config = NetConfig {
            blender_port: port,
            ..NetConfig::default()
        };
        let (tx, rx) = bounded(8);
        let shutdown = Arc::new(AtomicBool::new(false));
        let server = {
            let shutdown = shutdown.clone();
            tokio::spawn(async move {
                start_blender_server(&config, FrameSink::new(tx), shutdown).await
            })
        };

        let mut client = None;
        for _ in 0..50 {
            match tokio::net::TcpStream::connect(("127.0.0.1", port)).await {
                Ok(c) => {
                    client = Some(c);
                    break;
                }
                Err(_) => tokio::time::sleep(Duration::from_millis(20)).await,
            }
        }
        let mut client = client.expect("server never started listening");
        client.write_all(&addon_line()).await.unwrap();

        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        let frame = loop {
            if let Ok(frame) = rx.try_recv() {
                break frame;
            }
            assert!(std::time::Instant::now() < deadline, "no frame received");
            tokio::time::sleep(Duration::from_millis(10)).await;
        };
        assert_eq!(endpoints(&frame), L_SHAPE);

        client.write_all(b"CLOSE\n").await.unwrap();
        shutdown.store(true, Ordering::Relaxed);
        tokio::time::timeout(Duration::from_secs(2), server)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}
