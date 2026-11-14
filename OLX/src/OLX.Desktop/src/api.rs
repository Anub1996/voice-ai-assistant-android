use reqwest::blocking::Client;
use reqwest::header::{HeaderMap, AUTHORIZATION, CONTENT_TYPE};
use crate::models::*;

pub struct ApiClient {
    client: Client,
    base_url: String,
    token: Option<String>,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::builder().timeout(std::time::Duration::from_secs(15)).build().unwrap(),
            base_url: base_url.trim_end_matches('/').to_string(),
            token: None,
        }
    }

    pub fn set_token(&mut self, token: String) { self.token = Some(token); }
    pub fn clear_token(&mut self) { self.token = None; }
    pub fn has_token(&self) -> bool { self.token.is_some() }

    fn headers(&self) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(CONTENT_TYPE, "application/json".parse().unwrap());
        if let Some(t) = &self.token {
            h.insert(AUTHORIZATION, format!("Bearer {}", t).parse().unwrap());
        }
        h
    }

    fn url(&self, path: &str) -> String { format!("{}{}", self.base_url, path) }

    // Auth
    pub fn login(&self, username: &str, password: &str) -> Result<LoginResponse, String> {
        let body = LoginRequest { username: username.to_string(), password: password.to_string() };
        self.client.post(self.url("/api/auth/login")).headers(self.headers()).json(&body).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    pub fn register(&self, req: &RegisterRequest) -> Result<LoginResponse, String> {
        self.client.post(self.url("/api/auth/register")).headers(self.headers()).json(req).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    // Listings
    pub fn get_listings(&self, page: i32, page_size: i32, search: Option<&str>, category_id: Option<i32>, sort: Option<&str>, condition: Option<&str>) -> Result<PagedResult<ListingItem>, String> {
        let mut url = format!("{}/api/listings?page={}&pageSize={}", self.base_url, page, page_size);
        if let Some(s) = search { url.push_str(&format!("&search={}", s)); }
        if let Some(c) = category_id { url.push_str(&format!("&categoryId={}", c)); }
        if let Some(s) = sort { url.push_str(&format!("&sort={}", s)); }
        if let Some(c) = condition { url.push_str(&format!("&condition={}", c)); }
        self.client.get(&url).headers(self.headers()).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    pub fn get_featured(&self) -> Result<Vec<ListingItem>, String> {
        self.client.get(self.url("/api/listings/featured")).headers(self.headers()).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    pub fn get_listing(&self, id: i64) -> Result<ListingDetail, String> {
        self.client.get(self.url(&format!("/api/listings/{}", id))).headers(self.headers()).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    pub fn get_my_listings(&self) -> Result<Vec<ListingItem>, String> {
        self.client.get(self.url("/api/listings/mine")).headers(self.headers()).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    pub fn create_listing(&self, req: &CreateListingRequest) -> Result<IdResponse, String> {
        self.client.post(self.url("/api/listings")).headers(self.headers()).json(req).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    // Categories
    pub fn get_categories(&self) -> Result<Vec<Category>, String> {
        self.client.get(self.url("/api/categories")).headers(self.headers()).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    // Chat
    pub fn get_conversations(&self) -> Result<Vec<Conversation>, String> {
        self.client.get(self.url("/api/chat/conversations")).headers(self.headers()).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    pub fn get_messages(&self, conversation_id: i64) -> Result<Vec<ChatMessage>, String> {
        self.client.get(self.url(&format!("/api/chat/conversations/{}/messages", conversation_id))).headers(self.headers()).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    pub fn start_conversation(&self, listing_id: i64, message: &str) -> Result<IdResponse, String> {
        let body = serde_json::json!({"listingId": listing_id, "message": message});
        self.client.post(self.url("/api/chat/conversations")).headers(self.headers()).json(&body).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    pub fn send_message(&self, conversation_id: i64, text: &str) -> Result<MessageResponse, String> {
        let body = serde_json::json!({"text": text});
        self.client.post(self.url(&format!("/api/chat/conversations/{}/messages", conversation_id))).headers(self.headers()).json(&body).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    // Favorites
    pub fn get_favorites(&self) -> Result<Vec<FavoriteItem>, String> {
        self.client.get(self.url("/api/favorites")).headers(self.headers()).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    pub fn add_favorite(&self, listing_id: i64) -> Result<MessageResponse, String> {
        self.client.post(self.url(&format!("/api/favorites/{}", listing_id))).headers(self.headers()).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    pub fn remove_favorite(&self, listing_id: i64) -> Result<MessageResponse, String> {
        self.client.delete(self.url(&format!("/api/favorites/{}", listing_id))).headers(self.headers()).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    // Notifications
    pub fn get_notifications(&self) -> Result<Vec<NotificationItem>, String> {
        self.client.get(self.url("/api/notifications")).headers(self.headers()).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    pub fn get_unread_count(&self) -> Result<UnreadCount, String> {
        self.client.get(self.url("/api/notifications/unread-count")).headers(self.headers()).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    pub fn mark_notification_read(&self, id: i64) -> Result<MessageResponse, String> {
        self.client.put(self.url(&format!("/api/notifications/{}/read", id))).headers(self.headers()).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    pub fn mark_all_read(&self) -> Result<MessageResponse, String> {
        self.client.put(self.url("/api/notifications/read-all")).headers(self.headers()).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    // Profile
    pub fn get_profile(&self) -> Result<UserProfile, String> {
        self.client.get(self.url("/api/users/me")).headers(self.headers()).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    pub fn update_profile(&self, req: &UpdateProfileRequest) -> Result<MessageResponse, String> {
        self.client.put(self.url("/api/users/me")).headers(self.headers()).json(req).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    pub fn get_seller(&self, id: i32) -> Result<Seller, String> {
        self.client.get(self.url(&format!("/api/users/{}", id))).headers(self.headers()).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }

    // Reports
    pub fn report_listing(&self, listing_id: i64, reason: &str, description: &str) -> Result<MessageResponse, String> {
        let body = serde_json::json!({"reason": reason, "description": description});
        self.client.post(self.url(&format!("/api/reports/listings/{}", listing_id))).headers(self.headers()).json(&body).send()
            .map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())
    }
}
