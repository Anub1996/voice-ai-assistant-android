use eframe::egui;
use crate::models::*;
use crate::app::{AppState, Screen};

pub fn draw_profile(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if ui.button("< Back").clicked() {
            state.screen = Screen::Home;
        }
        ui.heading("My Profile");
    });
    ui.separator();

    if state.profile.is_none() {
        ui.label("Profile not loaded.");
        if ui.button("Load Profile").clicked() {
            match state.api.get_profile() {
                Ok(p) => {
                    state.edit_full_name = p.full_name.clone();
                    state.edit_phone = p.phone.clone();
                    state.edit_bio = p.bio.clone().unwrap_or_default();
                    state.edit_city = p.city.clone().unwrap_or_default();
                    state.edit_state = p.state.clone().unwrap_or_default();
                    state.profile = Some(p);
                }
                Err(e) => state.status_message = format!("Error: {}", e),
            }
        }
        return;
    }

    let profile = state.profile.clone().unwrap();

    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.group(|ui| {
            ui.heading("Account Info");
            ui.horizontal(|ui| {
                ui.label("Username:");
                ui.strong(&profile.username);
            });
            ui.horizontal(|ui| {
                ui.label("Full Name:");
                ui.strong(&profile.full_name);
            });
            ui.horizontal(|ui| {
                ui.label("Email:");
                ui.label(&profile.email);
            });
            ui.horizontal(|ui| {
                ui.label("Phone:");
                ui.label(&profile.phone);
            });
            if let Some(bio) = &profile.bio {
                if !bio.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label("Bio:");
                        ui.label(bio);
                    });
                }
            }
            ui.horizontal(|ui| {
                ui.label("Location:");
                let loc = format!(
                    "{}{}",
                    profile.city.as_deref().unwrap_or(""),
                    profile.state.as_ref().map(|s| format!(", {}", s)).unwrap_or_default()
                );
                ui.label(if loc.is_empty() { "Not set".to_string() } else { loc });
            });
            ui.horizontal(|ui| {
                ui.label("Joined:");
                ui.label(&profile.joined_at);
            });
            ui.horizontal(|ui| {
                ui.label("Verified:");
                ui.label(if profile.is_verified { "Yes" } else { "No" });
            });
            ui.horizontal(|ui| {
                ui.label("Listings:");
                ui.label(profile.listings_count.to_string());
            });
        });

        ui.add_space(10.0);

        ui.group(|ui| {
            ui.heading("Edit Profile");
            ui.horizontal(|ui| {
                ui.label("Full Name:");
                ui.text_edit_singleline(&mut state.edit_full_name);
            });
            ui.horizontal(|ui| {
                ui.label("Phone:");
                ui.text_edit_singleline(&mut state.edit_phone);
            });
            ui.horizontal(|ui| {
                ui.label("Bio:");
                ui.text_edit_singleline(&mut state.edit_bio);
            });
            ui.horizontal(|ui| {
                ui.label("City:");
                ui.text_edit_singleline(&mut state.edit_city);
            });
            ui.horizontal(|ui| {
                ui.label("State:");
                ui.text_edit_singleline(&mut state.edit_state);
            });
            if ui.button("Save Changes").clicked() {
                let req = UpdateProfileRequest {
                    full_name: Some(state.edit_full_name.clone()),
                    phone: Some(state.edit_phone.clone()),
                    bio: Some(state.edit_bio.clone()),
                    city: Some(state.edit_city.clone()),
                    state: Some(state.edit_state.clone()),
                };
                match state.api.update_profile(&req) {
                    Ok(resp) => {
                        state.status_message = resp.message;
                        state.full_name = state.edit_full_name.clone();
                        if let Ok(p) = state.api.get_profile() {
                            state.profile = Some(p);
                        }
                    }
                    Err(e) => state.status_message = format!("Error: {}", e),
                }
            }
        });

        ui.add_space(10.0);

        ui.group(|ui| {
            ui.heading("Navigation");
            ui.horizontal(|ui| {
                if ui.button("My Ads").clicked() {
                    state.screen = Screen::MyAds;
                }
                if ui.button("Favorites").clicked() {
                    state.screen = Screen::Favorites;
                }
                if ui.button("Notifications").clicked() {
                    state.screen = Screen::Notifications;
                }
                if ui.button("Chat").clicked() {
                    state.screen = Screen::ChatList;
                }
            });
        });

        ui.add_space(10.0);

        if ui.button("Logout").clicked() {
            state.api.clear_token();
            state.user_id = 0;
            state.username.clear();
            state.full_name.clear();
            state.profile = None;
            state.screen = Screen::Login;
            state.status_message = "Logged out.".to_string();
        }
    });
}
