pub mod effect_panel;
pub mod scope;
pub mod state;

pub use state::{EditorSharedState, EffectSnapshot, UiCommand, VisBuffer};

use nih_plug::prelude::*;
use nih_plug_egui::egui;
use state::EditorSharedState as SharedState;

/// References to the nih-plug parameters exposed to the editor.
pub struct OsciPluginParamRefs<'a> {
    pub volume: &'a FloatParam,
    pub frequency: &'a FloatParam,
    pub attack: &'a FloatParam,
    pub decay: &'a FloatParam,
    pub sustain: &'a FloatParam,
    pub release: &'a FloatParam,
}

/// Draw the complete plugin editor UI.
///
/// Call this from within the `nih_plug_egui::create_egui_editor` update closure.
pub fn draw_editor(
    ui: &mut egui::Ui,
    params: &OsciPluginParamRefs,
    setter: &ParamSetter,
    shared: &SharedState,
    effect_snapshots: &[EffectSnapshot],
    vis: &VisBuffer,
    selected_effect_id: &mut String,
) {
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

        // XY Scope
        ui.heading("XY Scope");
        ui.separator();
        scope::draw_scope(ui, vis);
    });
}
