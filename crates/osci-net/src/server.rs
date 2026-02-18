use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crate::blender::start_blender_server;
use crate::config::NetConfig;
use crate::frame_channel::FrameSink;
use crate::websocket::start_ws_server;

/// Orchestrates Blender TCP and WebSocket servers on a background thread.
pub struct NetServer {
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl NetServer {
    /// Start both network servers on a background thread with a dedicated tokio runtime.
    pub fn start(config: NetConfig, sink: FrameSink) -> Self {
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        let thread = thread::Builder::new()
            .name("osci-net".to_string())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        log::error!("Failed to create tokio runtime: {}", e);
                        return;
                    }
                };

                rt.block_on(async {
                    let blender_sink = FrameSink::new(sink.sender());
                    let ws_sink = FrameSink::new(sink.sender());
                    let shutdown_b = shutdown_clone.clone();
                    let shutdown_w = shutdown_clone.clone();

                    tokio::select! {
                        result = start_blender_server(&config, blender_sink, shutdown_b) => {
                            if let Err(e) = result {
                                log::error!("Blender server error: {}", e);
                            }
                        }
                        result = start_ws_server(&config, ws_sink, shutdown_w) => {
                            if let Err(e) = result {
                                log::error!("WebSocket server error: {}", e);
                            }
                        }
                    }
                });
            })
            .expect("Failed to spawn osci-net thread");

        Self {
            shutdown,
            thread: Some(thread),
        }
    }

    /// Signal shutdown and wait for the background thread to finish.
    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for NetServer {
    fn drop(&mut self) {
        self.stop();
    }
}
