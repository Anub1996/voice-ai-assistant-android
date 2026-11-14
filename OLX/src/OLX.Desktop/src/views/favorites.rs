use eframe::egui;
use egui::{Color32, RichText, ScrollArea};
use crate::app::{AppState, Screen};

pub fn draw_favorites(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if ui.button("< Back").clicked() {
            state.screen = Screen::Home;
        }
        ui.heading("My Favorites");
    });

    ui.separator();

    if state.favorites.is_empty() {
        ui.add_space(20.0);
        ui.label(RichText::new("No favorites yet.").color(Color32::GRAY).size(16.0));
        ui.label("Browse listings and add items to your favorites.");
        return;
    }

    let mut remove_id: Option<i64> = None;
    let mut view_id: Option<i64> = None;

    ScrollArea::vertical().show(ui, |ui| {
        for fav in &state.favorites {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        if ui.link(RichText::new(&fav.title).strong()).clicked() {
                            view_id = Some(fav.listing_id);
                        }
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("\u{20b9}{:.0}", fav.price)).color(Color32::from_rgb(0, 128, 0)).strong());
                            ui.label(RichText::new(format!("  {}", fav.status)).color(Color32::GRAY));
                        });
                        if let Some(city) = &fav.city {
                            ui.label(RichText::new(city).color(Color32::GRAY));
                        }
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(RichText::new("Remove").color(Color32::RED)).clicked() {
                            remove_id = Some(fav.listing_id);
                        }
                    });
                });
            });
            ui.add_space(2.0);
        }
    });

    if let Some(id) = remove_id {
        let _ = state.api.remove_favorite(id);
        state.favorites.retain(|f| f.listing_id != id);
    }

    if let Some(id) = view_id {
        if let Ok(detail) = state.api.get_listing(id) {
            state.current_listing = Some(detail);
            state.screen = Screen::ListingDetail;
        }
    }
}
