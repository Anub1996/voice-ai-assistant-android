use eframe::egui;
use crate::app::{AppState, Screen};

pub fn draw_my_ads(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if ui.button("< Back").clicked() {
            state.screen = Screen::Profile;
        }
        ui.heading("My Ads");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("+ Post New Ad").clicked() {
                state.screen = Screen::PostListing;
            }
            if ui.button("Refresh").clicked() {
                match state.api.get_my_listings() {
                    Ok(items) => {
                        state.listings = items;
                        state.status_message = "Listings refreshed.".to_string();
                    }
                    Err(e) => state.status_message = format!("Error: {}", e),
                }
            }
        });
    });
    ui.separator();

    if state.listings.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.heading("No ads yet");
            ui.label("You haven't posted any ads. Tap 'Post New Ad' to get started.");
        });
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        let mut selected_id: Option<i64> = None;

        egui::Grid::new("my_ads_grid")
            .num_columns(4)
            .spacing([12.0, 8.0])
            .striped(true)
            .show(ui, |ui| {
                ui.strong("Title");
                ui.strong("Price");
                ui.strong("Status");
                ui.strong("Views");
                ui.end_row();

                for item in &state.listings {
                    if ui.link(&item.title).clicked() {
                        selected_id = Some(item.id);
                    }

                    ui.label(format!("\u{20b9}{:.0}", item.price));

                    let status_color = match item.status.as_str() {
                        "active" => egui::Color32::from_rgb(0, 160, 0),
                        "sold" => egui::Color32::from_rgb(0, 100, 200),
                        "expired" => egui::Color32::from_rgb(200, 100, 0),
                        _ => egui::Color32::GRAY,
                    };
                    ui.colored_label(status_color, &item.status);

                    ui.label(format!("{}", item.view_count));
                    ui.end_row();
                }
            });

        if let Some(id) = selected_id {
            if let Ok(detail) = state.api.get_listing(id) {
                state.current_listing = Some(detail);
                state.screen = Screen::ListingDetail;
            }
        }
    });
}
