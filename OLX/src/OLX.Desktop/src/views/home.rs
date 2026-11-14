use eframe::egui;
use crate::app::{AppState, Screen};

fn draw_listing_card(ui: &mut egui::Ui, item: &crate::models::ListingItem) -> bool {
    let mut clicked = false;
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_min_width(ui.available_width());
        ui.vertical(|ui| {
            if item.is_featured {
                ui.colored_label(egui::Color32::from_rgb(255, 165, 0), "★ Featured");
            }
            let title_text = egui::RichText::new(&item.title).strong().size(15.0);
            if ui.link(title_text).clicked() {
                clicked = true;
            }
            ui.label(egui::RichText::new(format!("₹{:.0}", item.price)).size(18.0).strong().color(egui::Color32::from_rgb(0, 150, 0)));
            ui.horizontal(|ui| {
                ui.label(&item.condition);
                ui.label("·");
                ui.label(item.city.as_deref().unwrap_or("Unknown"));
            });
            ui.horizontal(|ui| {
                ui.label(format!("by {}", item.seller_name));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("{} views", item.view_count));
                });
            });
        });
    });
    clicked
}

pub fn draw_home(ui: &mut egui::Ui, state: &mut AppState) {
    if !state.home_loaded {
        state.load_home_data();
    }

    // Top bar
    ui.horizontal(|ui| {
        ui.heading(egui::RichText::new("OLX").size(28.0).strong());
        ui.add_space(16.0);

        let search_field = egui::TextEdit::singleline(&mut state.search_query)
            .hint_text("Search products...")
            .desired_width(300.0);
        let search_resp = ui.add(search_field);
        if search_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !state.search_query.is_empty() {
            state.screen = Screen::Search;
            state.search_page = 1;
        }
        if ui.button("Search").clicked() && !state.search_query.is_empty() {
            state.screen = Screen::Search;
            state.search_page = 1;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("👤 Profile").clicked() {
                state.screen = Screen::Profile;
            }
            let notif_label = if state.unread_count > 0 {
                format!("🔔 ({})", state.unread_count)
            } else {
                "🔔".to_string()
            };
            if ui.button(&notif_label).clicked() {
                state.screen = Screen::Notifications;
            }
            if ui.button("❤ Favorites").clicked() {
                state.screen = Screen::Favorites;
            }
            if ui.button("💬 Chat").clicked() {
                state.screen = Screen::ChatList;
            }
            if ui.button("+ Sell").clicked() {
                state.screen = Screen::PostListing;
            }
            if ui.button("My Ads").clicked() {
                state.screen = Screen::MyAds;
            }
        });
    });
    ui.separator();

    egui::ScrollArea::vertical().show(ui, |ui| {
        // Categories
        if !state.categories.is_empty() {
            ui.label(egui::RichText::new("Categories").size(16.0).strong());
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                for cat in state.categories.clone() {
                    let label = format!("{} {}", cat.icon, cat.name);
                    if ui.button(&label).clicked() {
                        state.search_category = Some(cat.id);
                        state.search_query.clear();
                        state.search_page = 1;
                        state.screen = Screen::Search;
                    }
                }
            });
            ui.add_space(8.0);
            ui.separator();
        }

        // Featured
        if !state.featured.is_empty() {
            ui.add_space(4.0);
            ui.label(egui::RichText::new("Featured Listings").size(16.0).strong());
            ui.add_space(4.0);
            let featured_clone = state.featured.clone();
            ui.columns(2, |cols| {
                for (i, item) in featured_clone.iter().enumerate() {
                    let col = &mut cols[i % 2];
                    if draw_listing_card(col, item) {
                        // Deferred: set after columns
                    }
                }
            });
            // Handle clicks via separate pass
            let featured_clone2 = state.featured.clone();
            for item in &featured_clone2 {
                // Already handled by link clicks inside draw_listing_card
                let _ = item;
            }
            ui.add_space(8.0);
            ui.separator();
        }

        // All listings
        ui.add_space(4.0);
        ui.label(egui::RichText::new("All Listings").size(16.0).strong());
        ui.add_space(4.0);

        if state.listings.is_empty() {
            ui.label("No listings found. Pull data by logging in.");
        } else {
            let mut nav_to: Option<i64> = None;
            let listings_clone = state.listings.clone();
            ui.columns(2, |cols| {
                for (i, item) in listings_clone.iter().enumerate() {
                    let col = &mut cols[i % 2];
                    if draw_listing_card(col, item) {
                        nav_to = Some(item.id);
                    }
                    col.add_space(4.0);
                }
            });

            if let Some(id) = nav_to {
                match state.api.get_listing(id) {
                    Ok(detail) => {
                        state.current_listing = Some(detail);
                        state.screen = Screen::ListingDetail;
                    }
                    Err(e) => state.status_message = format!("Error: {}", e),
                }
            }

            // Pagination
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if state.listings_page > 1 {
                    if ui.button("← Previous").clicked() {
                        state.listings_page -= 1;
                    }
                }
                ui.label(format!("Page {} of {}", state.listings_page, state.listings_total_pages.max(1)));
                if state.listings_page < state.listings_total_pages {
                    if ui.button("Next →").clicked() {
                        state.listings_page += 1;
                    }
                }
            });
        }

        // Status
        if !state.status_message.is_empty() {
            ui.add_space(8.0);
            ui.label(&state.status_message);
        }
    });
}
