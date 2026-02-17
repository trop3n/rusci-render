use osci_core::EffectParameter;
use serde::{Deserialize, Serialize};
use std::io;
use std::path::Path;

/// On-disk project file format.
#[derive(Serialize, Deserialize)]
pub struct ProjectFile {
    pub version: u32,
    pub synth: SynthParamSnapshot,
    pub effects: Vec<EffectStateEntry>,
    #[serde(default)]
    pub visualizer: Option<VisualizerSnapshot>,
}

/// Snapshot of synthesizer parameters.
#[derive(Serialize, Deserialize)]
pub struct SynthParamSnapshot {
    pub volume: f32,
    pub frequency: f32,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

/// One effect in the saved chain.
#[derive(Serialize, Deserialize)]
pub struct EffectStateEntry {
    pub id: String,
    pub enabled: bool,
    pub parameters: Vec<EffectParameter>,
}

/// Snapshot of visualizer settings.
#[derive(Serialize, Deserialize)]
pub struct VisualizerSnapshot {
    pub focus: f32,
    pub intensity: f32,
    pub persistence: f32,
    pub afterglow: f32,
    pub glow_amount: f32,
    pub scatter_amount: f32,
    pub color: [f32; 3],
    pub exposure: f32,
    pub overexposure: f32,
    pub saturation: f32,
    pub ambient: f32,
    pub noise: f32,
}

/// Save a project file to disk as JSON.
pub fn save_project(path: &Path, project: &ProjectFile) -> io::Result<()> {
    let json = serde_json::to_string_pretty(project)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, json)
}

/// Load a project file from disk.
pub fn load_project(path: &Path) -> io::Result<ProjectFile> {
    let json = std::fs::read_to_string(path)?;
    serde_json::from_str(&json).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
