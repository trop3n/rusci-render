use nih_plug_egui::egui;

/// Tracks which dialogs are currently open.
#[derive(Default)]
pub struct MenuState {
    pub show_about: bool,
    pub show_audio_info: bool,
    pub show_shortcuts: bool,
}

/// Actions returned from the menu bar that require processing by the caller.
#[derive(Debug, PartialEq)]
pub enum MenuAction {
    None,
    NewProject,
    OpenProject,
    SaveProject,
    SaveProjectAs,
}

/// Draw the menu bar inside a `TopBottomPanel`. Returns a `MenuAction` if a file
/// operation was requested.
pub fn draw_menu_bar(ui: &mut egui::Ui, state: &mut MenuState) -> MenuAction {
    let mut action = MenuAction::None;

    egui::menu::bar(ui, |ui| {
        ui.menu_button("File", |ui| {
            if ui
                .add(egui::Button::new("New Project").shortcut_text("Ctrl+N"))
                .clicked()
            {
                action = MenuAction::NewProject;
                ui.close_menu();
            }
            ui.separator();
            if ui
                .add(egui::Button::new("Open Project...").shortcut_text("Ctrl+O"))
                .clicked()
            {
                action = MenuAction::OpenProject;
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("Save Project").shortcut_text("Ctrl+S"))
                .clicked()
            {
                action = MenuAction::SaveProject;
                ui.close_menu();
            }
            if ui
                .add(egui::Button::new("Save Project As...").shortcut_text("Ctrl+Shift+S"))
                .clicked()
            {
                action = MenuAction::SaveProjectAs;
                ui.close_menu();
            }
        });

        ui.menu_button("Audio", |ui| {
            if ui.button("Device Info...").clicked() {
                state.show_audio_info = true;
                ui.close_menu();
            }
        });

        ui.menu_button("Help", |ui| {
            if ui.button("Keyboard Shortcuts").clicked() {
                state.show_shortcuts = true;
                ui.close_menu();
            }
            if ui.button("About rusci-render").clicked() {
                state.show_about = true;
                ui.close_menu();
            }
        });
    });

    action
}
