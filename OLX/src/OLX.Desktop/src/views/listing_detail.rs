use eframe::egui;
use egui::{Color32, RichText, ScrollArea};
use crate::app::{AppState, Screen};

pub fn draw_listing_detail(ui: &mut egui::Ui, state: &mut AppState) {
    let listing = match &state.current_listing {
        Some(l) => l.clone(),
        None => {
            ui.label("No listing selected.");
            if ui.button("Back").clicked() {
                state.screen = Screen::Home;
            }
            return;
        }
    };

    ui.horizontal(|ui| {
        if ui.button("< Back").clicked() {
            state.current_listing = None;
            state.screen = Screen::Home;
        }
        ui.heading("Listing Details");
    });

    ui.separator();

    ScrollArea::vertical().show(ui, |ui| {
        ui.heading(RichText::new(&listing.title).size(24.0));
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("\u{20b9}{:.0}", listing.price)).size(22.0).color(Color32::from_rgb(0, 128, 0)).strong());
            if listing.is_negotiable {
                ui.label(RichText::new(" Negotiable").color(Color32::from_rgb(0, 100, 200)).italics());
            }
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let cond_color = match listing.condition.as_str() {
                "new" => Color32::from_rgb(0, 160, 0),
                "like_new" => Color32::from_rgb(0, 128, 128),
                "used" => Color32::from_rgb(180, 140, 0),
                _ => Color32::from_rgb(180, 80, 0),
            };
            ui.label(RichText::new(format!("Condition: {}", listing.condition.replace('_', " "))).color(cond_color).strong());
            if listing.is_featured {
                ui.label(RichText::new(" FEATURED").color(Color32::from_rgb(255, 165, 0)).strong());
            }
        });

        ui.add_space(4.0);
        ui.label(RichText::new(format!("Category: {}", listing.category.name)).color(Color32::GRAY));

        if let Some(city) = &listing.city {
            let loc = match &listing.state {
                Some(s) => format!("{}, {}", city, s),
                None => city.clone(),
            };
            ui.label(RichText::new(format!("Location: {}", loc)).color(Color32::GRAY));
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        ui.label(RichText::new("Description").strong().size(16.0));
        ui.label(&listing.description);

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        ui.label(RichText::new("Seller").strong().size(16.0));
        ui.group(|ui| {
            ui.label(RichText::new(&listing.seller.full_name).strong());
            if listing.seller.is_verified {
                ui.label(RichText::new("Verified").color(Color32::from_rgb(0, 128, 0)));
            }
            if let Some(city) = &listing.seller.city {
                ui.label(format!("City: {}", city));
            }
            ui.label(format!("Joined: {}", &listing.seller.joined_at[..10]));
        });

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(format!("Views: {}", listing.view_count));
            ui.label(format!("Favorites: {}", listing.favorite_count));
            ui.label(format!("Listed: {}", &listing.created_at[..10]));
        });

        ui.add_space(12.0);

        if !state.status_message.is_empty() {
            ui.label(RichText::new(&state.status_message).color(Color32::from_rgb(0, 128, 0)));
            ui.add_space(4.0);
        }

        ui.horizontal(|ui| {
            if ui.button(RichText::new("Add to Favorites").strong()).clicked() {
                match state.api.add_favorite(listing.id) {
                    Ok(_) => state.status_message = "Added to favorites!".to_string(),
                    Err(e) => state.status_message = format!("Error: {}", e),
                }
            }

            if listing.seller.id != state.user_id {
                if ui.button(RichText::new("Chat with Seller").strong()).clicked() {
                    match state.api.start_conversation(listing.id, "Hi, I'm interested in this item.") {
                        Ok(resp) => {
                            state.current_conversation = Some(resp.id);
                            if let Ok(msgs) = state.api.get_messages(resp.id) {
                                state.messages = msgs;
                            }
                            state.screen = Screen::Chat;
                        }
                        Err(e) => state.status_message = format!("Error: {}", e),
                    }
                }
            }
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button("View My Ads").clicked() {
                if let Ok(items) = state.api.get_my_listings() {
                    state.listings = items;
                }
                state.screen = Screen::MyAds;
            }
        });
    });
}
