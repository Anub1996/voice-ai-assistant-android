use serde::{Deserialize, Serialize};

// Auth
#[derive(Serialize)]
pub struct LoginRequest { pub username: String, pub password: String }
#[derive(Serialize)]
pub struct RegisterRequest { pub username: String, pub email: String, pub phone: String, pub password: String, #[serde(rename = "fullName")] pub full_name: String }
#[derive(Deserialize, Clone, Default)]
pub struct LoginResponse { pub token: String, #[serde(rename = "userId")] pub user_id: i32, pub username: String, #[serde(rename = "fullName")] pub full_name: String, #[serde(rename = "profilePhotoUrl")] pub profile_photo_url: Option<String> }

// User
#[derive(Deserialize, Clone, Default)]
pub struct UserProfile {
    pub id: i32, pub username: String, #[serde(rename = "fullName")] pub full_name: String,
    pub email: String, pub phone: String,
    #[serde(rename = "profilePhotoUrl")] pub profile_photo_url: Option<String>,
    pub bio: Option<String>,
    pub city: Option<String>, pub state: Option<String>,
    #[serde(rename = "joinedAt")] pub joined_at: String,
    #[serde(rename = "isVerified")] pub is_verified: bool,
    #[serde(rename = "listingsCount")] pub listings_count: i32,
    #[serde(rename = "avgResponseTime")] pub avg_response_time: f64,
}
#[derive(Serialize)]
pub struct UpdateProfileRequest { #[serde(rename = "fullName")] pub full_name: Option<String>, pub phone: Option<String>, pub bio: Option<String>, pub city: Option<String>, pub state: Option<String> }

#[derive(Deserialize, Clone, Default)]
pub struct Seller {
    pub id: i32, pub username: String, #[serde(rename = "fullName")] pub full_name: String,
    #[serde(rename = "profilePhotoUrl")] pub profile_photo_url: Option<String>,
    pub city: Option<String>, #[serde(rename = "joinedAt")] pub joined_at: String,
    #[serde(rename = "isVerified")] pub is_verified: bool,
}

// Category
#[derive(Deserialize, Clone, Default)]
pub struct Category { pub id: i32, pub name: String, pub slug: String, pub icon: String, #[serde(rename = "parentId")] pub parent_id: Option<i32>, pub children: Option<Vec<Category>> }

// Listing
#[derive(Serialize)]
pub struct CreateListingRequest {
    pub title: String, pub description: String, pub price: f64, pub currency: Option<String>,
    #[serde(rename = "isNegotiable")] pub is_negotiable: bool, pub condition: String,
    #[serde(rename = "categoryId")] pub category_id: i32,
    pub city: Option<String>, pub state: Option<String>,
}

#[derive(Deserialize, Clone, Default)]
pub struct ListingItem {
    pub id: i64, pub title: String, pub slug: String, pub price: f64, pub currency: String,
    #[serde(rename = "isNegotiable")] pub is_negotiable: bool, pub condition: String,
    pub city: Option<String>, pub state: Option<String>, pub status: String,
    #[serde(rename = "isFeatured")] pub is_featured: bool,
    #[serde(rename = "viewCount")] pub view_count: i32,
    #[serde(rename = "thumbnailUrl")] pub thumbnail_url: Option<String>,
    #[serde(rename = "categoryName")] pub category_name: String,
    #[serde(rename = "sellerName")] pub seller_name: String,
    #[serde(rename = "sellerId")] pub seller_id: i32,
    #[serde(rename = "createdAt")] pub created_at: String,
    #[serde(rename = "isFavorited")] pub is_favorited: bool,
}

#[derive(Deserialize, Clone, Default)]
pub struct ListingDetail {
    pub id: i64, pub title: String, pub description: String, pub price: f64, pub currency: String,
    #[serde(rename = "isNegotiable")] pub is_negotiable: bool, pub condition: String,
    pub category: Category, pub seller: Seller,
    pub city: Option<String>, pub state: Option<String>, pub status: String,
    #[serde(rename = "isFeatured")] pub is_featured: bool,
    #[serde(rename = "viewCount")] pub view_count: i32,
    pub images: Vec<String>,
    #[serde(rename = "createdAt")] pub created_at: String,
    #[serde(rename = "isFavorited")] pub is_favorited: bool,
    #[serde(rename = "favoriteCount")] pub favorite_count: i32,
}

#[derive(Deserialize, Clone)]
pub struct PagedResult<T> { pub data: Vec<T>, pub total: i32, pub page: i32, #[serde(rename = "pageSize")] pub page_size: i32, #[serde(rename = "totalPages")] pub total_pages: i32 }

// Chat
#[derive(Deserialize, Clone, Default)]
pub struct Conversation {
    pub id: i64, #[serde(rename = "listingId")] pub listing_id: i64,
    #[serde(rename = "listingTitle")] pub listing_title: String,
    #[serde(rename = "listingPrice")] pub listing_price: f64,
    #[serde(rename = "otherUserId")] pub other_user_id: i32,
    #[serde(rename = "otherUserName")] pub other_user_name: String,
    #[serde(rename = "lastMessage")] pub last_message: Option<String>,
    #[serde(rename = "lastMessageAt")] pub last_message_at: Option<String>,
    #[serde(rename = "unreadCount")] pub unread_count: i32,
}

#[derive(Deserialize, Clone)]
pub struct ChatMessage {
    pub id: i64, #[serde(rename = "senderId")] pub sender_id: i32,
    #[serde(rename = "senderName")] pub sender_name: String,
    pub text: String, #[serde(rename = "isRead")] pub is_read: bool,
    #[serde(rename = "sentAt")] pub sent_at: String,
}

// Favorite
#[derive(Deserialize, Clone)]
pub struct FavoriteItem {
    #[serde(rename = "listingId")] pub listing_id: i64, pub title: String,
    pub price: f64, pub currency: String, pub city: Option<String>,
    pub status: String, #[serde(rename = "favoritedAt")] pub favorited_at: String,
}

// Notification
#[derive(Deserialize, Clone)]
pub struct NotificationItem {
    pub id: i64, #[serde(rename = "type")] pub ntype: String,
    pub title: String, pub body: String,
    #[serde(rename = "listingId")] pub listing_id: Option<i64>,
    #[serde(rename = "isRead")] pub is_read: bool,
    #[serde(rename = "createdAt")] pub created_at: String,
}

#[derive(Deserialize)]
pub struct MessageResponse { pub message: String }
#[derive(Deserialize)]
pub struct IdResponse { pub message: String, pub id: i64 }
#[derive(Deserialize)]
pub struct UnreadCount { pub count: i32 }
