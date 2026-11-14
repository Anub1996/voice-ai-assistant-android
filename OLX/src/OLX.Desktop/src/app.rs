use eframe::egui;
use crate::api::ApiClient;
use crate::models::*;
use crate::views;

#[derive(PartialEq, Clone)]
pub enum Screen {
    Login, Register, Home, Search, ListingDetail, PostListing,
    ChatList, Chat, Favorites, Notifications, Profile, MyAds,
}

pub struct AppState {
    pub api: ApiClient,
    pub screen: Screen,
    pub user_id: i32,
    pub username: String,
    pub full_name: String,
    // Login
    pub login_username: String,
    pub login_password: String,
    pub login_error: String,
    // Register
    pub reg_username: String,
    pub reg_email: String,
    pub reg_phone: String,
    pub reg_password: String,
    pub reg_full_name: String,
    pub reg_error: String,
    // Home
    pub categories: Vec<Category>,
    pub listings: Vec<ListingItem>,
    pub featured: Vec<ListingItem>,
    pub listings_page: i32,
    pub listings_total_pages: i32,
    pub home_loaded: bool,
    // Search
    pub search_query: String,
    pub search_results: Vec<ListingItem>,
    pub search_category: Option<i32>,
    pub search_sort: String,
    pub search_condition: String,
    pub search_page: i32,
    pub search_total_pages: i32,
    // Detail
    pub current_listing: Option<ListingDetail>,
    // Post
    pub post_title: String,
    pub post_description: String,
    pub post_price: String,
    pub post_category: i32,
    pub post_condition: String,
    pub post_city: String,
    pub post_state: String,
    pub post_negotiable: bool,
    pub post_message: String,
    // Chat
    pub conversations: Vec<Conversation>,
    pub current_conversation: Option<i64>,
    pub messages: Vec<ChatMessage>,
    pub chat_input: String,
    // Favorites
    pub favorites: Vec<FavoriteItem>,
    // Notifications
    pub notifications: Vec<NotificationItem>,
    pub unread_count: i32,
    // Profile
    pub profile: Option<UserProfile>,
    pub edit_full_name: String,
    pub edit_phone: String,
    pub edit_bio: String,
    pub edit_city: String,
    pub edit_state: String,
    // General
    pub status_message: String,
    pub loading: bool,
}

impl AppState {
    pub fn new(base_url: &str) -> Self {
        Self {
            api: ApiClient::new(base_url),
            screen: Screen::Login,
            user_id: 0, username: String::new(), full_name: String::new(),
            login_username: String::new(), login_password: String::new(), login_error: String::new(),
            reg_username: String::new(), reg_email: String::new(), reg_phone: String::new(),
            reg_password: String::new(), reg_full_name: String::new(), reg_error: String::new(),
            categories: Vec::new(), listings: Vec::new(), featured: Vec::new(),
            listings_page: 1, listings_total_pages: 1, home_loaded: false,
            search_query: String::new(), search_results: Vec::new(), search_category: None,
            search_sort: "newest".to_string(), search_condition: String::new(),
            search_page: 1, search_total_pages: 1,
            current_listing: None,
            post_title: String::new(), post_description: String::new(), post_price: String::new(),
            post_category: 0, post_condition: "used".to_string(),
            post_city: String::new(), post_state: String::new(),
            post_negotiable: false, post_message: String::new(),
            conversations: Vec::new(), current_conversation: None,
            messages: Vec::new(), chat_input: String::new(),
            favorites: Vec::new(),
            notifications: Vec::new(), unread_count: 0,
            profile: None,
            edit_full_name: String::new(), edit_phone: String::new(),
            edit_bio: String::new(), edit_city: String::new(), edit_state: String::new(),
            status_message: String::new(), loading: false,
        }
    }

    pub fn load_home_data(&mut self) {
        if let Ok(cats) = self.api.get_categories() {
            self.categories = cats;
        }
        if let Ok(result) = self.api.get_listings(self.listings_page, 20, None, None, None, None) {
            self.listings = result.data;
            self.listings_total_pages = result.total_pages;
        }
        if let Ok(feat) = self.api.get_featured() {
            self.featured = feat;
        }
        if let Ok(uc) = self.api.get_unread_count() {
            self.unread_count = uc.count;
        }
        self.home_loaded = true;
    }

    pub fn do_search(&mut self) {
        let search = if self.search_query.is_empty() { None } else { Some(self.search_query.as_str()) };
        let sort = if self.search_sort.is_empty() { None } else { Some(self.search_sort.as_str()) };
        let cond = if self.search_condition.is_empty() { None } else { Some(self.search_condition.as_str()) };
        if let Ok(result) = self.api.get_listings(self.search_page, 20, search, self.search_category, sort, cond) {
            self.search_results = result.data;
            self.search_total_pages = result.total_pages;
        }
    }
}

pub struct OlxApp {
    pub state: AppState,
}

impl OlxApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self { state: AppState::new("http://localhost:5000") }
    }
}

impl eframe::App for OlxApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if !self.state.status_message.is_empty() {
                ui.colored_label(egui::Color32::from_rgb(0, 150, 0), &self.state.status_message);
                ui.separator();
            }

            // Bottom nav for logged-in screens
            let screen = self.state.screen.clone();
            if screen != Screen::Login && screen != Screen::Register {
                egui::TopBottomPanel::bottom("nav").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.selectable_label(screen == Screen::Home, "Home").clicked() {
                            self.state.screen = Screen::Home;
                            self.state.home_loaded = false;
                        }
                        ui.separator();
                        if ui.selectable_label(screen == Screen::ChatList, "Chat").clicked() {
                            self.state.screen = Screen::ChatList;
                            self.state.current_conversation = None;
                            if let Ok(c) = self.state.api.get_conversations() { self.state.conversations = c; }
                        }
                        ui.separator();
                        if ui.selectable_label(screen == Screen::PostListing, "Sell").clicked() {
                            self.state.screen = Screen::PostListing;
                        }
                        ui.separator();
                        if ui.selectable_label(screen == Screen::Favorites, "Favorites").clicked() {
                            self.state.screen = Screen::Favorites;
                            if let Ok(f) = self.state.api.get_favorites() { self.state.favorites = f; }
                        }
                        ui.separator();
                        if ui.selectable_label(screen == Screen::Profile, "Profile").clicked() {
                            self.state.screen = Screen::Profile;
                            if let Ok(p) = self.state.api.get_profile() {
                                self.state.edit_full_name = p.full_name.clone();
                                self.state.edit_phone = p.phone.clone();
                                self.state.edit_bio = p.bio.clone().unwrap_or_default();
                                self.state.edit_city = p.city.clone().unwrap_or_default();
                                self.state.edit_state = p.state.clone().unwrap_or_default();
                                self.state.profile = Some(p);
                            }
                        }
                    });
                });
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.state.screen.clone() {
                    Screen::Login => views::login::draw_login(ui, &mut self.state),
                    Screen::Register => views::register::draw_register(ui, &mut self.state),
                    Screen::Home => views::home::draw_home(ui, &mut self.state),
                    Screen::Search => views::search::draw_search(ui, &mut self.state),
                    Screen::ListingDetail => views::listing_detail::draw_listing_detail(ui, &mut self.state),
                    Screen::PostListing => views::post_listing::draw_post_listing(ui, &mut self.state),
                    Screen::ChatList => views::chat::draw_chat(ui, &mut self.state),
                    Screen::Chat => views::chat::draw_chat(ui, &mut self.state),
                    Screen::Favorites => views::favorites::draw_favorites(ui, &mut self.state),
                    Screen::Notifications => views::notifications::draw_notifications(ui, &mut self.state),
                    Screen::Profile => views::profile::draw_profile(ui, &mut self.state),
                    Screen::MyAds => views::my_ads::draw_my_ads(ui, &mut self.state),
                }
            });
        });
    }
}
