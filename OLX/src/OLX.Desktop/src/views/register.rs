use eframe::egui;
use crate::app::{AppState, Screen};
use crate::models::RegisterRequest;

pub fn draw_register(ui: &mut egui::Ui, state: &mut AppState) {
    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.heading(egui::RichText::new("Create Account").size(32.0).strong());
        ui.add_space(24.0);

        let panel_width = 320.0;
        ui.allocate_ui(egui::vec2(panel_width, 0.0), |ui| {
            ui.vertical(|ui| {
                if !state.reg_error.is_empty() {
                    ui.colored_label(egui::Color32::RED, &state.reg_error);
                    ui.add_space(8.0);
                }

                ui.label("Full Name");
                ui.text_edit_singleline(&mut state.reg_full_name);
                ui.add_space(6.0);

                ui.label("Username");
                ui.text_edit_singleline(&mut state.reg_username);
                ui.add_space(6.0);

                ui.label("Email");
                ui.text_edit_singleline(&mut state.reg_email);
                ui.add_space(6.0);

                ui.label("Phone");
                ui.text_edit_singleline(&mut state.reg_phone);
                ui.add_space(6.0);

                ui.label("Password");
                let pw = egui::TextEdit::singleline(&mut state.reg_password).password(true);
                ui.add(pw);
                ui.add_space(16.0);

                let reg_btn = ui.add_sized([panel_width, 36.0], egui::Button::new(
                    egui::RichText::new("Register").size(16.0),
                ));
                if reg_btn.clicked() {
                    state.reg_error.clear();
                    let req = RegisterRequest {
                        username: state.reg_username.clone(),
                        email: state.reg_email.clone(),
                        phone: state.reg_phone.clone(),
                        password: state.reg_password.clone(),
                        full_name: state.reg_full_name.clone(),
                    };
                    match state.api.register(&req) {
                        Ok(resp) => {
                            state.api.set_token(resp.token);
                            state.user_id = resp.user_id;
                            state.username = resp.username;
                            state.full_name = resp.full_name;
                            state.screen = Screen::Home;
                            state.status_message = "Account created".to_string();
                        }
                        Err(e) => {
                            state.reg_error = format!("Registration failed: {}", e);
                        }
                    }
                }

                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.label("Already have an account?");
                    if ui.link("Login").clicked() {
                        state.reg_error.clear();
                        state.screen = Screen::Login;
                    }
                });
            });
        });
    });
}
