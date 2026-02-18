use crossbeam::channel::Sender;
use osci_core::shape::Shape;

/// Wraps a crossbeam sender so network servers can push shape frames
/// to the audio thread without blocking.
pub struct FrameSink {
    tx: Sender<Vec<Box<dyn Shape>>>,
}

impl FrameSink {
    pub fn new(tx: Sender<Vec<Box<dyn Shape>>>) -> Self {
        Self { tx }
    }

    /// Non-blocking send. Returns `true` if the frame was accepted.
    pub fn send(&self, frame: Vec<Box<dyn Shape>>) -> bool {
        self.tx.try_send(frame).is_ok()
    }

    /// Clone the underlying sender for use in spawned tasks.
    pub fn sender(&self) -> Sender<Vec<Box<dyn Shape>>> {
        self.tx.clone()
    }
}
