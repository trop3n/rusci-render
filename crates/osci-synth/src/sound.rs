use crossbeam::channel::{bounded, Receiver, Sender, TryRecvError};
use osci_core::shape::Shape;

/// A frame is a collection of shapes representing one animation frame.
pub type Frame = Vec<Box<dyn Shape>>;

/// ShapeSound — manages a queue of frames for a voice to consume.
///
/// Mirrors the C++ `ShapeSound` class. Frames are produced by a parser/producer
/// on a background thread and consumed by the voice on the audio thread.
pub struct ShapeSound {
    frame_rx: Receiver<Frame>,
    frame_tx: Sender<Frame>,
    current_frame: Frame,
    frame_length: f64,
}

impl ShapeSound {
    /// Create a new ShapeSound with the given queue capacity.
    pub fn new(capacity: usize) -> Self {
        let (tx, rx) = bounded(capacity);
        Self {
            frame_rx: rx,
            frame_tx: tx,
            current_frame: Vec::new(),
            frame_length: 0.0,
        }
    }

    /// Get a sender handle for the frame producer.
    pub fn sender(&self) -> Sender<Frame> {
        self.frame_tx.clone()
    }

    /// Try to update the current frame from the queue.
    ///
    /// Returns the total length of the new frame's shapes. If no new frame is
    /// available, returns the length of the current frame.
    pub fn update_frame(&mut self) -> f64 {
        match self.frame_rx.try_recv() {
            Ok(frame) => {
                self.frame_length = osci_core::shape::total_length(&frame) as f64;
                self.current_frame = frame;
                self.frame_length
            }
            Err(TryRecvError::Empty) => self.frame_length,
            Err(TryRecvError::Disconnected) => self.frame_length,
        }
    }

    /// Clone the current frame's shapes for use by a voice.
    pub fn clone_frame(&self) -> Frame {
        self.current_frame
            .iter()
            .map(|s| s.clone_shape())
            .collect()
    }

    /// Get the current frame length.
    pub fn frame_length(&self) -> f64 {
        self.frame_length
    }

    /// Check if the current frame is empty.
    pub fn is_empty(&self) -> bool {
        self.current_frame.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use osci_core::shape::Line;
    use osci_core::Point;

    #[test]
    fn test_sound_frame_queue() {
        let mut sound = ShapeSound::new(4);
        let tx = sound.sender();

        let line = Line::from_points(
            Point::new(0.0, 0.0, 0.0),
            Point::new(1.0, 0.0, 0.0),
        );
        let frame: Frame = vec![Box::new(line)];
        tx.send(frame).unwrap();

        let len = sound.update_frame();
        assert!(len > 0.0);
        assert!(!sound.is_empty());
    }

    #[test]
    fn test_sound_empty() {
        let mut sound = ShapeSound::new(4);
        let len = sound.update_frame();
        assert!((len).abs() < 0.001);
        assert!(sound.is_empty());
    }
}
