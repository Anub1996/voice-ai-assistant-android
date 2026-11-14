use eframe::egui;
use egui::{Color32, RichText, ScrollArea};
use crate::app::{AppState, Screen};

pub fn draw_notifications(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if ui.button("< Back").clicked() {
            state.screen = Screen::Home;
        }
        ui.heading("Notifications");
        if state.unread_count > 0 {
            ui.label(RichText::new(format!("({} unread)", state.unread_count)).color(Color32::RED));
        }
    });

    ui.separator();

    if !state.notifications.is_empty() {
        ui.horizontal(|ui| {
            if ui.button("Mark All Read").clicked() {
                if state.api.mark_all_read().is_ok() {
                    for n in &mut state.notifications {
                        n.is_read = true;
                    }
                    state.unread_count = 0;
                }
            }
        });
        ui.add_space(4.0);
    }

    if state.notifications.is_empty() {
        ui.add_space(20.0);
        ui.label(RichText::new("No notifications.").color(Color32::GRAY).size(16.0));
        ui.label("You'll be notified about messages, price drops, and more.");
        return;
    }

    ScrollArea::vertical().show(ui, |ui| {
        let notifs = state.notifications.clone();
        for (idx, notif) in notifs.iter().enumerate() {
            let icon = match notif.ntype.as_str() {
                "message" => "💬",
                "price_drop" => "💰",
                "new_listing" => "🆕",
                "favorite_sold" => "❤️",
                "listing_expired" => "⏰",
                "listing_approved" => "✅",
                _ => "🔔",
            };

            let response = ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(icon).size(18.0));
                    ui.vertical(|ui| {
                        let title_text = if notif.is_read {
                            RichText::new(&notif.title)
                        } else {
                            RichText::new(&notif.title).strong()
                        };
                        ui.label(title_text);
                        ui.label(RichText::new(&notif.body).color(Color32::GRAY));
                        let time_str = if notif.created_at.len() >= 10 { &notif.created_at[..10] } else { &notif.created_at };
                        ui.label(RichText::new(time_str).small().color(Color32::DARK_GRAY));
                    });
                    if !notif.is_read {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("●").color(Color32::from_rgb(0, 100, 200)));
                        });
                    }
                });
            }).response.interact(egui::Sense::click());

            if response.clicked() && !notif.is_read {
                let _ = state.api.mark_notification_read(notif.id);
                if idx < state.notifications.len() {
                    state.notifications[idx].is_read = true;
                    state.unread_count = (state.unread_count - 1).max(0);
                }
            }

            ui.add_space(2.0);
        }
    });
}
