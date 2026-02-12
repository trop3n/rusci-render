//! Frame producer — background thread that generates shape frames
//! and feeds them to the audio thread via a bounded channel.
//!
//! Mirrors the C++ `FrameProducer` class. A parser produces frames on a
//! background thread, the audio thread consumes them from a `ShapeSound`
//! queue without blocking.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crossbeam::channel::Sender;
use osci_core::shape::Shape;

/// A frame is a vector of boxed shapes.
pub type Frame = Vec<Box<dyn Shape>>;

/// A function that produces the next frame of shapes.
///
/// Implementations should block if necessary (e.g. waiting for animation
/// timing) but must check `should_stop` periodically and return `None`
/// when it's time to shut down.
pub trait FrameSource: Send + 'static {
    /// Generate the next frame. Returns `None` to signal shutdown.
    fn next_frame(&mut self) -> Option<Frame>;
}

/// A simple frame source that repeatedly clones a static set of shapes.
pub struct StaticFrameSource {
    shapes: Frame,
}

impl StaticFrameSource {
    pub fn new(shapes: Frame) -> Self {
        Self { shapes }
    }
}

impl FrameSource for StaticFrameSource {
    fn next_frame(&mut self) -> Option<Frame> {
        let cloned: Frame = self.shapes.iter().map(|s| s.clone_shape()).collect();
        Some(cloned)
    }
}

/// A frame source that cycles through pre-parsed animation frames.
pub struct AnimatedFrameSource {
    frames: Vec<Frame>,
    current_frame: usize,
    frame_rate: f64,
}

impl AnimatedFrameSource {
    pub fn new(frames: Vec<Frame>, frame_rate: f64) -> Self {
        Self {
            frames,
            current_frame: 0,
            frame_rate,
        }
    }

    pub fn set_frame(&mut self, index: usize) {
        if !self.frames.is_empty() {
            self.current_frame = index % self.frames.len();
        }
    }
}

impl FrameSource for AnimatedFrameSource {
    fn next_frame(&mut self) -> Option<Frame> {
        if self.frames.is_empty() {
            return None;
        }
        let frame: Frame = self.frames[self.current_frame]
            .iter()
            .map(|s| s.clone_shape())
            .collect();
        self.current_frame = (self.current_frame + 1) % self.frames.len();
        Some(frame)
    }
}

/// Background frame producer thread.
///
/// Continuously generates frames from a `FrameSource` and sends them
/// through a crossbeam channel to the audio thread's `ShapeSound`.
pub struct FrameProducer {
    running: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl FrameProducer {
    /// Start producing frames in a background thread.
    ///
    /// Frames are sent to `frame_tx`. The producer runs until `stop()` is
    /// called or the channel is disconnected.
    pub fn start(mut source: impl FrameSource, frame_tx: Sender<Frame>) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        let thread = thread::Builder::new()
            .name("frame-producer".to_string())
            .spawn(move || {
                while running_clone.load(Ordering::Relaxed) {
                    match source.next_frame() {
                        Some(frame) => {
                            // Send will block if the queue is full, providing
                            // natural backpressure
                            if frame_tx.send(frame).is_err() {
                                // Channel disconnected, stop producing
                                break;
                            }
                        }
                        None => {
                            // Source exhausted, stop
                            break;
                        }
                    }
                }
            })
            .expect("failed to spawn frame producer thread");

        Self {
            running,
            thread: Some(thread),
        }
    }

    /// Signal the producer to stop and wait for it to finish.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }

    /// Check if the producer is still running.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }
}

impl Drop for FrameProducer {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam::channel::bounded;
    use osci_core::shape::Line;

    #[test]
    fn test_static_frame_source() {
        let shapes: Frame = vec![
            Box::new(Line::new_2d(0.0, 0.0, 1.0, 1.0)),
        ];
        let mut source = StaticFrameSource::new(shapes);
        let frame = source.next_frame().unwrap();
        assert_eq!(frame.len(), 1);
    }

    #[test]
    fn test_animated_frame_source() {
        let frame1: Frame = vec![Box::new(Line::new_2d(0.0, 0.0, 1.0, 0.0))];
        let frame2: Frame = vec![Box::new(Line::new_2d(0.0, 0.0, 0.0, 1.0))];
        let mut source = AnimatedFrameSource::new(vec![frame1, frame2], 30.0);

        let f1 = source.next_frame().unwrap();
        let f2 = source.next_frame().unwrap();
        let f3 = source.next_frame().unwrap(); // wraps back to frame 0

        // f1 and f3 should be the same (both frame 0)
        let p1 = f1[0].next_vector(1.0);
        let p3 = f3[0].next_vector(1.0);
        assert!((p1.x - p3.x).abs() < 0.001);
    }

    #[test]
    fn test_frame_producer_lifecycle() {
        let shapes: Frame = vec![
            Box::new(Line::new_2d(-1.0, -1.0, 1.0, 1.0)),
        ];
        let source = StaticFrameSource::new(shapes);
        let (tx, rx) = bounded(4);

        let mut producer = FrameProducer::start(source, tx);

        // Should receive frames
        let frame = rx.recv().unwrap();
        assert_eq!(frame.len(), 1);

        // Stop the producer
        producer.stop();
        assert!(!producer.is_running());
    }
}
