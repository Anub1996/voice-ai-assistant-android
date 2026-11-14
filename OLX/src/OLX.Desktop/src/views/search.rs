use eframe::egui;
use crate::app::{AppState, Screen};

pub fn draw_search(ui: &mut egui::Ui, state: &mut AppState) {
    // Top bar
    ui.horizontal(|ui| {
        if ui.button("← Back").clicked() {
            state.screen = Screen::Home;
        }
        ui.heading(egui::RichText::new("Search").size(22.0).strong());
    });
    ui.add_space(4.0);

    // Search bar
    ui.horizontal(|ui| {
        let field = egui::TextEdit::singleline(&mut state.search_query)
            .hint_text("Search...")
            .desired_width(300.0);
        let resp = ui.add(field);
        if ui.button("Search").clicked() || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
            state.search_page = 1;
            do_search(state);
        }
    });
    ui.add_space(4.0);

    // Filters
    ui.horizontal(|ui| {
        ui.label("Sort:");
        for (label, val) in [("Newest", "newest"), ("Price ↑", "price_low"), ("Price ↓", "price_high")] {
            let selected = state.search_sort == val;
            let btn = if selected {
                ui.button(egui::RichText::new(label).strong().underline())
            } else {
                ui.button(label)
            };
            if btn.clicked() {
                state.search_sort = val.to_string();
                state.search_page = 1;
                do_search(state);
            }
        }

        ui.add_space(16.0);
        ui.label("Condition:");
        for (label, val) in [("All", ""), ("New", "new"), ("Like New", "like_new"), ("Used", "used"), ("Fair", "fair")] {
            let selected = state.search_condition == val;
            let btn = if selected {
                ui.button(egui::RichText::new(label).strong().underline())
            } else {
                ui.button(label)
            };
            if btn.clicked() {
                state.search_condition = val.to_string();
                state.search_page = 1;
                do_search(state);
            }
        }
    });
    ui.separator();

    // Results
    egui::ScrollArea::vertical().show(ui, |ui| {
        if state.search_results.is_empty() {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label("No results found.");
            });
        } else {
            let mut nav_to: Option<i64> = None;
            let results_clone = state.search_results.clone();
            ui.columns(2, |cols| {
                for (i, item) in results_clone.iter().enumerate() {
                    let col = &mut cols[i % 2];
                    egui::Frame::group(col.style()).show(col, |ui| {
                        ui.set_min_width(ui.available_width());
                        ui.vertical(|ui| {
                            if item.is_featured {
                                ui.colored_label(egui::Color32::from_rgb(255, 165, 0), "★ Featured");
                            }
                            let title = egui::RichText::new(&item.title).strong().size(15.0);
                            if ui.link(title).clicked() {
                                nav_to = Some(item.id);
                            }
                            ui.label(egui::RichText::new(format!("₹{:.0}", item.price)).size(18.0).strong().color(egui::Color32::from_rgb(0, 150, 0)));
                            ui.horizontal(|ui| {
                                ui.label(&item.condition);
                                ui.label("·");
                                ui.label(item.city.as_deref().unwrap_or("Unknown"));
                            });
                            ui.label(format!("by {}", item.seller_name));
                        });
                    });
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
                if state.search_page > 1 {
                    if ui.button("← Previous").clicked() {
                        state.search_page -= 1;
                        do_search(state);
                    }
                }
                ui.label(format!("Page {} of {}", state.search_page, state.search_total_pages.max(1)));
                if state.search_page < state.search_total_pages {
                    if ui.button("Next →").clicked() {
                        state.search_page += 1;
                        do_search(state);
                    }
                }
            });
        }
    });
}

fn do_search(state: &mut AppState) {
    let search = if state.search_query.is_empty() { None } else { Some(state.search_query.as_str()) };
    let sort = if state.search_sort.is_empty() { None } else { Some(state.search_sort.as_str()) };
    let cond = if state.search_condition.is_empty() { None } else { Some(state.search_condition.as_str()) };
    match state.api.get_listings(state.search_page, 20, search, state.search_category, sort, cond) {
        Ok(result) => {
            state.search_results = result.data;
            state.search_total_pages = result.total_pages;
        }
        Err(e) => state.status_message = format!("Search error: {}", e),
    }
}
