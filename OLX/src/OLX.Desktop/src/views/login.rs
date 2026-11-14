use eframe::egui;
use crate::app::{AppState, Screen};

pub fn draw_login(ui: &mut egui::Ui, state: &mut AppState) {
    ui.vertical_centered(|ui| {
        ui.add_space(80.0);
        ui.heading(egui::RichText::new("OLX").size(48.0).strong());
        ui.add_space(8.0);
        ui.label("Buy & Sell Near You");
        ui.add_space(40.0);

        let panel_width = 320.0;
        ui.allocate_ui(egui::vec2(panel_width, 0.0), |ui| {
            ui.vertical(|ui| {
                if !state.login_error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &state.login_error);
                    ui.add_space(8.0);
                }

                ui.label("Username");
                ui.text_edit_singleline(&mut state.login_username);
                ui.add_space(8.0);

                ui.label("Password");
                let pw = egui::TextEdit::singleline(&mut state.login_password).password(true);
                ui.add(pw);
                ui.add_space(16.0);

                let login_btn = ui.add_sized([panel_width, 36.0], egui::Button::new(
                    egui::RichText::new("Login").size(16.0),
                ));
                if login_btn.clicked() {
                    state.login_error.clear();
                    match state.api.login(&state.login_username, &state.login_password) {
                        Ok(resp) => {
                            state.api.set_token(resp.token);
                            state.user_id = resp.user_id;
                            state.username = resp.username;
                            state.full_name = resp.full_name;
                            state.screen = Screen::Home;
                            state.status_message = "Logged in successfully".to_string();
                        }
                        Err(e) => {
                            state.login_error = format!("Login failed: {}", e);
                        }
                    }
                }

                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.label("Don't have an account?");
                    if ui.link("Register").clicked() {
                        state.login_error.clear();
                        state.screen = Screen::Register;
                    }
                });
            });
        });
    });
}
