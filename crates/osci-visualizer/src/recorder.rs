use std::path::PathBuf;
use std::sync::mpsc::{self, SyncSender};
use std::thread::{self, JoinHandle};

/// A single captured frame of pixel data.
pub struct CapturedFrame {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Configuration for video recording.
pub struct RecordConfig {
    pub output_path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub bitrate: usize,
}

/// Handle to a running recording session.
pub struct RecorderHandle {
    frame_tx: Option<SyncSender<CapturedFrame>>,
    thread: Option<JoinHandle<Result<(), String>>>,
}

impl RecorderHandle {
    /// Start a recording session. Spawns an encoder thread.
    pub fn start(config: RecordConfig) -> Result<Self, String> {
        let (tx, rx) = mpsc::sync_channel::<CapturedFrame>(4);

        let thread = thread::Builder::new()
            .name("osci-recorder".to_string())
            .spawn(move || encode_loop(config, rx))
            .map_err(|e| format!("Failed to spawn recorder thread: {}", e))?;

        Ok(Self {
            frame_tx: Some(tx),
            thread: Some(thread),
        })
    }

    /// Get a sender for submitting captured frames.
    pub fn sender(&self) -> Option<SyncSender<CapturedFrame>> {
        self.frame_tx.clone()
    }

    /// Stop recording, flush the encoder, and finalize the output file.
    pub fn stop(&mut self) -> Result<(), String> {
        // Drop the sender to signal the encoder thread to finish
        self.frame_tx.take();
        if let Some(handle) = self.thread.take() {
            handle
                .join()
                .map_err(|_| "Recorder thread panicked".to_string())?
        } else {
            Ok(())
        }
    }
}

#[cfg(feature = "video")]
fn encode_loop(
    config: RecordConfig,
    rx: mpsc::Receiver<CapturedFrame>,
) -> Result<(), String> {
    ffmpeg_next::init().map_err(|e| format!("ffmpeg init failed: {}", e))?;

    let mut octx = ffmpeg_next::format::output(&config.output_path)
        .map_err(|e| format!("Failed to create output context: {}", e))?;

    let codec = ffmpeg_next::encoder::find(ffmpeg_next::codec::Id::H264)
        .ok_or_else(|| "H264 codec not found".to_string())?;

    let mut stream = octx
        .add_stream(codec)
        .map_err(|e| format!("Failed to add stream: {}", e))?;

    let mut encoder = stream
        .codec()
        .encoder()
        .video()
        .map_err(|e| format!("Failed to get video encoder: {}", e))?;

    encoder.set_width(config.width);
    encoder.set_height(config.height);
    encoder.set_format(ffmpeg_next::format::Pixel::YUV420P);
    encoder.set_time_base(ffmpeg_next::Rational::new(1, config.fps as i32));
    encoder.set_bit_rate(config.bitrate);

    let mut encoder = encoder
        .open_as(codec)
        .map_err(|e| format!("Failed to open encoder: {}", e))?;

    octx.write_header()
        .map_err(|e| format!("Failed to write header: {}", e))?;

    let mut sws_ctx = ffmpeg_next::software::scaling::Context::get(
        ffmpeg_next::format::Pixel::RGBA,
        config.width,
        config.height,
        ffmpeg_next::format::Pixel::YUV420P,
        config.width,
        config.height,
        ffmpeg_next::software::scaling::Flags::BILINEAR,
    )
    .map_err(|e| format!("Failed to create scaler: {}", e))?;

    let mut frame_idx: i64 = 0;

    while let Ok(captured) = rx.recv() {
        let mut src_frame = ffmpeg_next::frame::Video::new(
            ffmpeg_next::format::Pixel::RGBA,
            captured.width,
            captured.height,
        );
        src_frame.data_mut(0).copy_from_slice(&captured.pixels);

        let mut dst_frame = ffmpeg_next::frame::Video::new(
            ffmpeg_next::format::Pixel::YUV420P,
            config.width,
            config.height,
        );

        sws_ctx
            .run(&src_frame, &mut dst_frame)
            .map_err(|e| format!("Scaling failed: {}", e))?;

        dst_frame.set_pts(Some(frame_idx));
        frame_idx += 1;

        encoder
            .send_frame(&dst_frame)
            .map_err(|e| format!("Send frame failed: {}", e))?;

        let mut packet = ffmpeg_next::Packet::empty();
        while encoder.receive_packet(&mut packet).is_ok() {
            packet.set_stream(0);
            packet
                .write_interleaved(&mut octx)
                .map_err(|e| format!("Write packet failed: {}", e))?;
        }
    }

    // Flush encoder
    encoder
        .send_eof()
        .map_err(|e| format!("Send EOF failed: {}", e))?;

    let mut packet = ffmpeg_next::Packet::empty();
    while encoder.receive_packet(&mut packet).is_ok() {
        packet.set_stream(0);
        packet
            .write_interleaved(&mut octx)
            .map_err(|e| format!("Flush packet failed: {}", e))?;
    }

    octx.write_trailer()
        .map_err(|e| format!("Failed to write trailer: {}", e))?;

    log::info!("Recording saved to {:?}", config.output_path);
    Ok(())
}

#[cfg(not(feature = "video"))]
fn encode_loop(
    _config: RecordConfig,
    _rx: mpsc::Receiver<CapturedFrame>,
) -> Result<(), String> {
    Err("Video recording requires the 'video' feature (ffmpeg-next)".to_string())
}
