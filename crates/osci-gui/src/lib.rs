pub mod dialogs;
pub mod effect_panel;
pub mod menu_bar;
pub mod project;
pub mod scope;
pub mod state;
pub mod theme;

pub use menu_bar::MenuState;
pub use scope::GpuScopeState;
pub use state::{AudioInfo, EditorSharedState, EffectSnapshot, LoadedEffect, UiCommand, VisBuffer};

use menu_bar::MenuAction;
use nih_plug::prelude::*;
use nih_plug_egui::egui;
use state::EditorSharedState as SharedState;
use std::sync::{Arc, Mutex};

/// References to the nih-plug parameters exposed to the editor.
pub struct OsciPluginParamRefs<'a> {
    pub volume: &'a FloatParam,
    pub frequency: &'a FloatParam,
    pub attack: &'a FloatParam,
    pub decay: &'a FloatParam,
    pub sustain: &'a FloatParam,
    pub release: &'a FloatParam,
}

/// Check for keyboard shortcuts and return the corresponding menu action.
fn check_shortcuts(ctx: &egui::Context) -> MenuAction {
    let modifiers = ctx.input(|i| i.modifiers);
    if modifiers.ctrl || modifiers.mac_cmd {
        if modifiers.shift && ctx.input(|i| i.key_pressed(egui::Key::S)) {
            return MenuAction::SaveProjectAs;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::S)) {
            return MenuAction::SaveProject;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::O)) {
            return MenuAction::OpenProject;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::N)) {
            return MenuAction::NewProject;
        }
    }
    MenuAction::None
}

/// Collect the current synth parameter values into a snapshot.
fn snapshot_synth_params(
    params: &OsciPluginParamRefs,
) -> project::SynthParamSnapshot {
    project::SynthParamSnapshot {
        volume: params.volume.unmodulated_plain_value(),
        frequency: params.frequency.unmodulated_plain_value(),
        attack: params.attack.unmodulated_plain_value(),
        decay: params.decay.unmodulated_plain_value(),
        sustain: params.sustain.unmodulated_plain_value(),
        release: params.release.unmodulated_plain_value(),
    }
}

/// Apply synth params from a loaded project via the ParamSetter.
fn apply_synth_params(
    params: &OsciPluginParamRefs,
    setter: &ParamSetter,
    snap: &project::SynthParamSnapshot,
) {
    setter.set_parameter(params.volume, snap.volume);
    setter.set_parameter(params.frequency, snap.frequency);
    setter.set_parameter(params.attack, snap.attack);
    setter.set_parameter(params.decay, snap.decay);
    setter.set_parameter(params.sustain, snap.sustain);
    setter.set_parameter(params.release, snap.release);
}

/// Build a ProjectFile from the current state.
fn build_project_file(
    params: &OsciPluginParamRefs,
    effect_snapshots: &[EffectSnapshot],
    scope_state: &Arc<Mutex<GpuScopeState>>,
) -> project::ProjectFile {
    let visualizer = scope_state.lock().ok().map(|state| {
        let s = &state.settings;
        project::VisualizerSnapshot {
            focus: s.focus,
            intensity: s.intensity,
            persistence: s.persistence,
            afterglow: s.afterglow,
            glow_amount: s.glow_amount,
            scatter_amount: s.scatter_amount,
            color: s.color,
            exposure: s.exposure,
            overexposure: s.overexposure,
            saturation: s.saturation,
            ambient: s.ambient,
            noise: s.noise,
            afterglow_color: Some(s.afterglow_color),
            reflection_mode: Some(s.reflection_mode),
            goniometer: Some(s.goniometer),
        }
    });

    project::ProjectFile {
        version: 1,
        synth: snapshot_synth_params(params),
        effects: effect_snapshots
            .iter()
            .map(|e| project::EffectStateEntry {
                id: e.id.clone(),
                enabled: e.enabled,
                parameters: e.parameters.clone(),
            })
            .collect(),
        visualizer,
    }
}

/// Pick a path for saving via native file dialog.
#[cfg(feature = "file-dialog")]
fn pick_save_path() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .set_title("Save Project")
        .add_filter("osci-project", &["osci-project"])
        .save_file()
}

#[cfg(not(feature = "file-dialog"))]
fn pick_save_path() -> Option<std::path::PathBuf> {
    log::warn!("File dialogs not available (build with 'file-dialog' feature)");
    None
}

/// Pick a path for opening via native file dialog.
#[cfg(feature = "file-dialog")]
fn pick_open_path() -> Option<std::path::PathBuf> {
    rfd::FileDialog::new()
        .set_title("Open Project")
        .add_filter("osci-project", &["osci-project"])
        .pick_file()
}

#[cfg(not(feature = "file-dialog"))]
fn pick_open_path() -> Option<std::path::PathBuf> {
    log::warn!("File dialogs not available (build with 'file-dialog' feature)");
    None
}

/// Handle a save action (Save or Save As).
fn handle_save(
    params: &OsciPluginParamRefs,
    effect_snapshots: &[EffectSnapshot],
    shared: &SharedState,
    scope_state: &Arc<Mutex<GpuScopeState>>,
    force_dialog: bool,
) {
    let existing_path = shared
        .current_project_path
        .lock()
        .ok()
        .and_then(|p| p.clone());

    let path = if force_dialog || existing_path.is_none() {
        pick_save_path()
    } else {
        existing_path
    };

    if let Some(path) = path {
        let proj = build_project_file(params, effect_snapshots, scope_state);
        if let Err(e) = project::save_project(&path, &proj) {
            log::error!("Failed to save project: {}", e);
        } else {
            if let Ok(mut p) = shared.current_project_path.lock() {
                *p = Some(path);
            }
        }
    }
}

/// Handle the open action.
fn handle_open(
    params: &OsciPluginParamRefs,
    setter: &ParamSetter,
    shared: &SharedState,
    scope_state: &Arc<Mutex<GpuScopeState>>,
) {
    let path = pick_open_path();

    if let Some(path) = path {
        match project::load_project(&path) {
            Ok(proj) => {
                // Apply synth params on UI thread
                apply_synth_params(params, setter, &proj.synth);

                // Send effect chain to audio thread
                let effects: Vec<LoadedEffect> = proj
                    .effects
                    .into_iter()
                    .map(|e| LoadedEffect {
                        id: e.id,
                        enabled: e.enabled,
                        parameters: e.parameters,
                    })
                    .collect();
                let _ = shared.command_tx.try_send(UiCommand::LoadProject { effects });

                // Apply visualizer settings
                if let Some(vis) = &proj.visualizer {
                    if let Ok(mut state) = scope_state.lock() {
                        state.settings.focus = vis.focus;
                        state.settings.intensity = vis.intensity;
                        state.settings.persistence = vis.persistence;
                        state.settings.afterglow = vis.afterglow;
                        state.settings.glow_amount = vis.glow_amount;
                        state.settings.scatter_amount = vis.scatter_amount;
                        state.settings.color = vis.color;
                        state.settings.exposure = vis.exposure;
                        state.settings.overexposure = vis.overexposure;
                        state.settings.saturation = vis.saturation;
                        state.settings.ambient = vis.ambient;
                        state.settings.noise = vis.noise;
                        if let Some(c) = vis.afterglow_color {
                            state.settings.afterglow_color = c;
                        }
                        if let Some(m) = vis.reflection_mode {
                            state.settings.reflection_mode = m;
                        }
                        if let Some(g) = vis.goniometer {
                            state.settings.goniometer = g;
                        }
                    }
                }

                if let Ok(mut p) = shared.current_project_path.lock() {
                    *p = Some(path);
                }
            }
            Err(e) => {
                log::error!("Failed to load project: {}", e);
            }
        }
    }
}

/// Handle the new project action.
fn handle_new(shared: &SharedState) {
    let _ = shared.command_tx.try_send(UiCommand::ClearProject);
    if let Ok(mut p) = shared.current_project_path.lock() {
        *p = None;
    }
}

/// Draw the complete plugin editor UI.
///
/// Call this from within the `nih_plug_egui::create_egui_editor` update closure.
/// The `menu_state` must be persisted across frames by the caller.
pub fn draw_editor(
    egui_ctx: &egui::Context,
    params: &OsciPluginParamRefs,
    setter: &ParamSetter,
    shared: &SharedState,
    effect_snapshots: &[EffectSnapshot],
    vis: &VisBuffer,
    selected_effect_id: &mut String,
    scope_state: Arc<Mutex<GpuScopeState>>,
    menu_state: &mut MenuState,
) {
    // Apply Dracula theme + Fira Sans font (guarded by Once)
    theme::apply(egui_ctx);

    // Check keyboard shortcuts
    let shortcut_action = check_shortcuts(egui_ctx);

    // Menu bar at top
    let menu_action = egui::TopBottomPanel::top("menu_bar").show(egui_ctx, |ui| {
        menu_bar::draw_menu_bar(ui, menu_state)
    }).inner;

    // Use whichever action was triggered (shortcut takes priority)
    let action = if shortcut_action != MenuAction::None {
        shortcut_action
    } else {
        menu_action
    };

    // Process menu action
    match action {
        MenuAction::NewProject => handle_new(shared),
        MenuAction::OpenProject => handle_open(params, setter, shared, &scope_state),
        MenuAction::SaveProject => handle_save(params, effect_snapshots, shared, &scope_state, false),
        MenuAction::SaveProjectAs => handle_save(params, effect_snapshots, shared, &scope_state, true),
        MenuAction::None => {}
    }

    // Draw dialogs
    let audio_info = shared.audio_info.lock().ok().map(|i| i.clone()).unwrap_or_default();
    dialogs::draw_about_dialog(egui_ctx, &mut menu_state.show_about);
    dialogs::draw_audio_info_dialog(egui_ctx, &mut menu_state.show_audio_info, &audio_info);
    dialogs::draw_shortcuts_dialog(egui_ctx, &mut menu_state.show_shortcuts);

    // Main content
    egui::CentralPanel::default().show(egui_ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            // Synth Controls
            ui.heading("Synth Controls");
            ui.separator();

            ui.add(nih_plug_egui::widgets::ParamSlider::for_param(params.volume, setter));
            ui.add(nih_plug_egui::widgets::ParamSlider::for_param(params.frequency, setter));

            ui.add_space(8.0);
            ui.label("ADSR Envelope");
            ui.add(nih_plug_egui::widgets::ParamSlider::for_param(params.attack, setter));
            ui.add(nih_plug_egui::widgets::ParamSlider::for_param(params.decay, setter));
            ui.add(nih_plug_egui::widgets::ParamSlider::for_param(params.sustain, setter));
            ui.add(nih_plug_egui::widgets::ParamSlider::for_param(params.release, setter));

            ui.add_space(12.0);

            // Effect Chain
            effect_panel::draw_effect_chain(ui, effect_snapshots, &shared.command_tx, selected_effect_id);

            ui.add_space(12.0);

            // XY Scope (GPU-rendered)
            ui.heading("XY Scope");
            ui.separator();
            scope::draw_gpu_scope(ui, vis, scope_state);
        });
    });
}
