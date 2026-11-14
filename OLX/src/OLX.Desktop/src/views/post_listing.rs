use eframe::egui;
use egui::{Color32, RichText, ScrollArea};
use crate::models::*;
use crate::app::{AppState, Screen};

pub fn draw_post_listing(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if ui.button("< Back").clicked() {
            state.post_message.clear();
            state.screen = Screen::Home;
        }
        ui.heading("Post New Ad");
    });

    ui.separator();

    ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(8.0);

        ui.label(RichText::new("Title").strong());
        ui.text_edit_singleline(&mut state.post_title);
        ui.add_space(4.0);

        ui.label(RichText::new("Description").strong());
        ui.add(egui::TextEdit::multiline(&mut state.post_description).desired_rows(5).desired_width(f32::INFINITY));
        ui.add_space(4.0);

        ui.label(RichText::new("Price (\u{20b9})").strong());
        ui.text_edit_singleline(&mut state.post_price);
        ui.add_space(4.0);

        ui.label(RichText::new("Category").strong());
        let cats: Vec<(i32, String)> = flatten_categories(&state.categories);
        let current_cat_name = cats.iter().find(|(id, _)| *id == state.post_category).map(|(_, n)| n.as_str()).unwrap_or("Select category");
        egui::ComboBox::from_label("")
            .selected_text(current_cat_name)
            .show_ui(ui, |ui| {
                for (id, name) in &cats {
                    if ui.selectable_label(state.post_category == *id, name).clicked() {
                        state.post_category = *id;
                    }
                }
            });
        ui.add_space(4.0);

        ui.label(RichText::new("Condition").strong());
        let conditions = ["new", "like_new", "used", "fair"];
        egui::ComboBox::from_id_source("condition_combo")
            .selected_text(&state.post_condition)
            .show_ui(ui, |ui| {
                for c in &conditions {
                    if ui.selectable_label(state.post_condition == *c, *c).clicked() {
                        state.post_condition = c.to_string();
                    }
                }
            });
        ui.add_space(4.0);

        ui.label(RichText::new("City").strong());
        ui.text_edit_singleline(&mut state.post_city);
        ui.add_space(4.0);

        ui.label(RichText::new("State").strong());
        ui.text_edit_singleline(&mut state.post_state);
        ui.add_space(4.0);

        ui.checkbox(&mut state.post_negotiable, "Price is negotiable");
        ui.add_space(8.0);

        if !state.post_message.is_empty() {
            let color = if state.post_message.starts_with("Error") { Color32::RED } else { Color32::from_rgb(0, 128, 0) };
            ui.label(RichText::new(&state.post_message).color(color));
            ui.add_space(4.0);
        }

        let can_post = !state.post_title.is_empty() && !state.post_price.is_empty() && state.post_category > 0;
        if ui.add_enabled(can_post, egui::Button::new(RichText::new("Post Ad").strong())).clicked() {
            let price: f64 = state.post_price.parse().unwrap_or(0.0);
            if price <= 0.0 {
                state.post_message = "Error: Invalid price".to_string();
                return;
            }
            let req = CreateListingRequest {
                title: state.post_title.clone(),
                description: state.post_description.clone(),
                price,
                currency: Some("INR".to_string()),
                is_negotiable: state.post_negotiable,
                condition: state.post_condition.clone(),
                category_id: state.post_category,
                city: if state.post_city.is_empty() { None } else { Some(state.post_city.clone()) },
                state: if state.post_state.is_empty() { None } else { Some(state.post_state.clone()) },
            };
            match state.api.create_listing(&req) {
                Ok(_) => {
                    state.post_message = "Ad posted successfully!".to_string();
                    state.post_title.clear();
                    state.post_description.clear();
                    state.post_price.clear();
                    state.post_city.clear();
                    state.post_state.clear();
                    state.post_negotiable = false;
                    if let Ok(items) = state.api.get_my_listings() {
                        state.listings = items;
                    }
                    state.screen = Screen::MyAds;
                }
                Err(e) => state.post_message = format!("Error: {}", e),
            }
        }
    });
}

fn flatten_categories(cats: &[Category]) -> Vec<(i32, String)> {
    let mut result = Vec::new();
    for cat in cats {
        result.push((cat.id, cat.name.clone()));
        if let Some(children) = &cat.children {
            for child in children {
                result.push((child.id, format!("  {} > {}", cat.name, child.name)));
            }
        }
    }
    result
}
