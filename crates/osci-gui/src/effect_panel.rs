use crate::state::{EffectSnapshot, UiCommand};
use crossbeam::channel::Sender;
use nih_plug_egui::egui::{self, Ui};
use osci_core::LfoType;
use osci_effects::registry::build_registry;

/// Draw the full effect chain panel: list of effects + add-effect controls.
pub fn draw_effect_chain(
    ui: &mut Ui,
    snapshots: &[EffectSnapshot],
    tx: &Sender<UiCommand>,
    selected_effect_id: &mut String,
) {
    ui.heading("Effect Chain");
    ui.separator();

    if snapshots.is_empty() {
        ui.label("No effects. Use the dropdown below to add one.");
    }

    let num_effects = snapshots.len();

    for (idx, snap) in snapshots.iter().enumerate() {
        let header_id = ui.make_persistent_id(format!("effect_{}", idx));
        egui::CollapsingHeader::new(format!("{} — {}", idx + 1, snap.name))
            .id_salt(header_id)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Enable/disable
                    let mut enabled = snap.enabled;
                    if ui.checkbox(&mut enabled, "Enabled").changed() {
                        let _ = tx.try_send(UiCommand::SetEffectEnabled { idx, enabled });
                    }

                    // Move up
                    if idx > 0 && ui.button("Up").clicked() {
                        let _ = tx.try_send(UiCommand::MoveEffect {
                            from: idx,
                            to: idx - 1,
                        });
                    }

                    // Move down
                    if idx + 1 < num_effects && ui.button("Down").clicked() {
                        let _ = tx.try_send(UiCommand::MoveEffect {
                            from: idx,
                            to: idx + 1,
                        });
                    }

                    // Remove
                    if ui.button("Remove").clicked() {
                        let _ = tx.try_send(UiCommand::RemoveEffect(idx));
                    }
                });

                // Parameter controls
                for (param_idx, param) in snap.parameters.iter().enumerate() {
                    draw_param_controls(ui, idx, param_idx, param, tx);
                }
            });
    }

    ui.separator();

    // Add effect dropdown
    ui.horizontal(|ui| {
        let registry = build_registry();
        egui::ComboBox::from_label("Add Effect")
            .selected_text(
                registry
                    .iter()
                    .find(|e| e.id == selected_effect_id.as_str())
                    .map(|e| e.name)
                    .unwrap_or("Select..."),
            )
            .show_ui(ui, |ui| {
                for entry in &registry {
                    ui.selectable_value(selected_effect_id, entry.id.to_string(), entry.name);
                }
            });

        if ui.button("Add").clicked() && !selected_effect_id.is_empty() {
            let _ = tx.try_send(UiCommand::AddEffect(selected_effect_id.clone()));
        }
    });
}

/// Draw parameter controls for a single effect parameter.
fn draw_param_controls(
    ui: &mut Ui,
    effect_idx: usize,
    param_idx: usize,
    param: &osci_core::EffectParameter,
    tx: &Sender<UiCommand>,
) {
    ui.group(|ui| {
        // Value slider
        let mut value = param.value;
        let slider = if param.step > 0.0 {
            egui::Slider::new(&mut value, param.min..=param.max)
                .text(&param.name)
                .step_by(param.step as f64)
        } else {
            egui::Slider::new(&mut value, param.min..=param.max).text(&param.name)
        };
        if ui.add(slider).changed() {
            let _ = tx.try_send(UiCommand::SetParamValue {
                effect_idx,
                param_idx,
                value,
            });
        }

        // LFO controls (collapsible)
        egui::CollapsingHeader::new(format!("LFO##{}_{}", effect_idx, param_idx))
            .id_salt(format!("lfo_{}_{}", effect_idx, param_idx))
            .default_open(false)
            .show(ui, |ui| {
                let mut lfo_type = param.lfo_type;
                let mut rate = param.lfo_rate;
                let mut start = param.lfo_start_percent;
                let mut end = param.lfo_end_percent;

                let mut changed = false;

                // LFO type selector
                egui::ComboBox::from_label(format!("Type##lfo_{}_{}", effect_idx, param_idx))
                    .selected_text(lfo_type_name(lfo_type))
                    .show_ui(ui, |ui| {
                        for &lt in ALL_LFO_TYPES {
                            if ui
                                .selectable_value(&mut lfo_type, lt, lfo_type_name(lt))
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    });

                if ui
                    .add(egui::Slider::new(&mut rate, 0.0..=20.0).text("Rate (Hz)"))
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .add(egui::Slider::new(&mut start, 0.0..=1.0).text("Start %"))
                    .changed()
                {
                    changed = true;
                }
                if ui
                    .add(egui::Slider::new(&mut end, 0.0..=1.0).text("End %"))
                    .changed()
                {
                    changed = true;
                }

                if changed {
                    let _ = tx.try_send(UiCommand::SetLfo {
                        effect_idx,
                        param_idx,
                        lfo_type,
                        rate,
                        start,
                        end,
                    });
                }
            });

        // Smoothing slider
        let mut smooth = param.smooth_value_change;
        if ui
            .add(egui::Slider::new(&mut smooth, 0.0..=1.0).text("Smoothing"))
            .changed()
        {
            let _ = tx.try_send(UiCommand::SetSmoothing {
                effect_idx,
                param_idx,
                value: smooth,
            });
        }

        // Sidechain toggle
        let mut sidechain = param.sidechain_enabled;
        if ui.checkbox(&mut sidechain, "Sidechain").changed() {
            let _ = tx.try_send(UiCommand::SetSidechain {
                effect_idx,
                param_idx,
                enabled: sidechain,
            });
        }
    });
}

const ALL_LFO_TYPES: &[LfoType] = &[
    LfoType::Static,
    LfoType::Sine,
    LfoType::Square,
    LfoType::Seesaw,
    LfoType::Triangle,
    LfoType::Sawtooth,
    LfoType::ReverseSawtooth,
    LfoType::Noise,
];

fn lfo_type_name(t: LfoType) -> &'static str {
    match t {
        LfoType::Static => "Static",
        LfoType::Sine => "Sine",
        LfoType::Square => "Square",
        LfoType::Seesaw => "Seesaw",
        LfoType::Triangle => "Triangle",
        LfoType::Sawtooth => "Sawtooth",
        LfoType::ReverseSawtooth => "Reverse Saw",
        LfoType::Noise => "Noise",
    }
}
