use crossbeam::channel::Sender;
use osci_core::{EffectParameter, LfoType};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// A serializable snapshot of one effect for project load.
#[derive(Clone, Debug)]
pub struct LoadedEffect {
    pub id: String,
    pub enabled: bool,
    pub parameters: Vec<EffectParameter>,
}

/// Commands sent from the UI thread to the audio thread via a lock-free channel.
pub enum UiCommand {
    /// Add an effect by its registry id.
    AddEffect(String),
    /// Remove the effect at the given chain index.
    RemoveEffect(usize),
    /// Move an effect from one index to another.
    MoveEffect { from: usize, to: usize },
    /// Enable or disable an effect at the given index.
    SetEffectEnabled { idx: usize, enabled: bool },
    /// Set a parameter value on an effect.
    SetParamValue {
        effect_idx: usize,
        param_idx: usize,
        value: f32,
    },
    /// Configure LFO modulation for a parameter.
    SetLfo {
        effect_idx: usize,
        param_idx: usize,
        lfo_type: LfoType,
        rate: f32,
        start: f32,
        end: f32,
    },
    /// Set the smoothing amount for a parameter.
    SetSmoothing {
        effect_idx: usize,
        param_idx: usize,
        value: f32,
    },
    /// Enable or disable sidechain modulation for a parameter.
    SetSidechain {
        effect_idx: usize,
        param_idx: usize,
        enabled: bool,
    },
    /// Load a project: replace the entire effect chain with the given effects.
    LoadProject {
        effects: Vec<LoadedEffect>,
    },
    /// Clear the current project (remove all effects).
    ClearProject,
    /// Start video recording with the given output path and dimensions.
    StartRecording {
        path: PathBuf,
        width: u32,
        height: u32,
        fps: u32,
    },
    /// Stop video recording.
    StopRecording,
}

/// A lightweight, UI-readable mirror of one effect in the chain.
#[derive(Clone)]
pub struct EffectSnapshot {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub parameters: Vec<EffectParameter>,
}

/// Audio device information for display in the UI.
#[derive(Clone, Debug, Default)]
pub struct AudioInfo {
    pub sample_rate: f32,
    pub buffer_size: u32,
}

/// Downsampled XY output buffer for the oscilloscope widget.
pub struct VisBuffer {
    pub x: Vec<f32>,
    pub y: Vec<f32>,
}

impl VisBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            x: vec![0.0; capacity],
            y: vec![0.0; capacity],
        }
    }
}

impl Default for VisBuffer {
    fn default() -> Self {
        Self::new(512)
    }
}

/// All shared data passed from the plugin to the editor.
pub struct EditorSharedState {
    pub command_tx: Sender<UiCommand>,
    pub effect_snapshots: Arc<Mutex<Vec<EffectSnapshot>>>,
    pub vis_buffer: Arc<Mutex<VisBuffer>>,
    pub current_project_path: Arc<Mutex<Option<PathBuf>>>,
    pub audio_info: Arc<Mutex<AudioInfo>>,
}
