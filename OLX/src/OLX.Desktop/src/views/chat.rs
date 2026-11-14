use eframe::egui;
use egui::{Align, Color32, Layout, RichText, ScrollArea};
use crate::app::{AppState, Screen};

pub fn draw_chat(ui: &mut egui::Ui, state: &mut AppState) {
    match state.current_conversation {
        None => draw_chat_list(ui, state),
        Some(conv_id) => draw_messages(ui, state, conv_id),
    }
}

fn draw_chat_list(ui: &mut egui::Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        if ui.button("< Back").clicked() {
            state.screen = Screen::Home;
        }
        ui.heading("Messages");
    });

    ui.separator();

    if state.conversations.is_empty() {
        ui.add_space(20.0);
        ui.label(RichText::new("No conversations yet.").color(Color32::GRAY).size(16.0));
        ui.label("Start chatting by visiting a listing and clicking 'Chat with Seller'.");
        return;
    }

    ScrollArea::vertical().show(ui, |ui| {
        let convos = state.conversations.clone();
        for conv in &convos {
            let clicked = ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&conv.listing_title).strong());
                        ui.label(RichText::new(&conv.other_user_name).color(Color32::GRAY));
                        if let Some(msg) = &conv.last_message {
                            let preview = if msg.len() > 50 { format!("{}...", &msg[..50]) } else { msg.clone() };
                            ui.label(preview);
                        }
                    });
                    if conv.unread_count > 0 {
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(RichText::new(format!("{}", conv.unread_count)).color(Color32::WHITE).background_color(Color32::RED).strong());
                        });
                    }
                });
            }).response.interact(egui::Sense::click());

            if clicked.clicked() {
                state.current_conversation = Some(conv.id);
                if let Ok(msgs) = state.api.get_messages(conv.id) {
                    state.messages = msgs;
                }
            }
            ui.add_space(2.0);
        }
    });
}

fn draw_messages(ui: &mut egui::Ui, state: &mut AppState, conv_id: i64) {
    let conv_title = state.conversations.iter()
        .find(|c| c.id == conv_id)
        .map(|c| c.listing_title.clone())
        .unwrap_or_else(|| "Chat".to_string());

    ui.horizontal(|ui| {
        if ui.button("< Back").clicked() {
            state.current_conversation = None;
            if let Ok(convos) = state.api.get_conversations() {
                state.conversations = convos;
            }
            return;
        }
        ui.heading(&conv_title);
    });

    ui.separator();

    let my_id = state.user_id;

    ScrollArea::vertical().auto_shrink([false; 2]).stick_to_bottom(true).show(ui, |ui| {
        let msgs = state.messages.clone();
        for msg in &msgs {
            let is_mine = msg.sender_id == my_id;
            let align = if is_mine { Align::Max } else { Align::Min };

            ui.with_layout(Layout::top_down(align), |ui| {
                ui.group(|ui| {
                    if !is_mine {
                        ui.label(RichText::new(&msg.sender_name).small().color(Color32::GRAY));
                    }
                    let bg = if is_mine { Color32::from_rgb(0, 100, 200) } else { Color32::from_rgb(80, 80, 80) };
                    ui.label(RichText::new(&msg.text).color(Color32::WHITE).background_color(bg));
                    let time_str = if msg.sent_at.len() >= 16 { &msg.sent_at[11..16] } else { &msg.sent_at };
                    ui.label(RichText::new(time_str).small().color(Color32::GRAY));
                });
            });
            ui.add_space(2.0);
        }
    });

    ui.separator();
    ui.horizontal(|ui| {
        let resp = ui.text_edit_singleline(&mut state.chat_input);
        let send = ui.button("Send");
        if (send.clicked() || (resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))) && !state.chat_input.is_empty() {
            let text = state.chat_input.clone();
            state.chat_input.clear();
            if state.api.send_message(conv_id, &text).is_ok() {
                if let Ok(msgs) = state.api.get_messages(conv_id) {
                    state.messages = msgs;
                }
            }
        }
    });
}
