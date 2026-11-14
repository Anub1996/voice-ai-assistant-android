package com.olxclone.app.api

// Auth
data class RegisterRequest(val username: String, val email: String, val phone: String, val password: String, val fullName: String)
data class LoginRequest(val username: String, val password: String)
data class LoginResponse(val token: String, val userId: Int, val username: String, val fullName: String, val profilePhotoUrl: String?, val expiresAt: String)
data class DeviceTokenRequest(val token: String)
data class ChangePasswordRequest(val oldPassword: String, val newPassword: String)

// User
data class UserProfile(val id: Int, val username: String, val fullName: String, val email: String, val phone: String,
    val profilePhotoUrl: String?, val bio: String?, val city: String?, val state: String?,
    val joinedAt: String, val isVerified: Boolean, val listingsCount: Int, val avgResponseTime: Double)
data class UpdateProfileRequest(val fullName: String?, val phone: String?, val bio: String?, val city: String?, val state: String?)
data class Seller(val id: Int, val username: String, val fullName: String, val profilePhotoUrl: String?, val city: String?, val joinedAt: String, val isVerified: Boolean)

// Category
data class Category(val id: Int, val name: String, val slug: String, val icon: String, val parentId: Int?, val children: List<Category>?)

// Listing
data class CreateListingRequest(val title: String, val description: String, val price: Double, val currency: String?,
    val isNegotiable: Boolean, val condition: String, val categoryId: Int, val latitude: Double?, val longitude: Double?,
    val address: String?, val city: String?, val state: String?, val imageUrls: List<String>?)
data class UpdateListingRequest(val title: String?, val description: String?, val price: Double?,
    val isNegotiable: Boolean?, val condition: String?, val categoryId: Int?, val address: String?, val city: String?, val state: String?)

data class ListingItem(val id: Long, val title: String, val slug: String, val price: Double, val currency: String,
    val isNegotiable: Boolean, val condition: String, val city: String?, val state: String?, val status: String,
    val isFeatured: Boolean, val viewCount: Int, val thumbnailUrl: String?, val categoryName: String,
    val sellerName: String, val sellerId: Int, val createdAt: String, val isFavorited: Boolean)
data class ListingDetail(val id: Long, val title: String, val slug: String, val description: String, val price: Double,
    val currency: String, val isNegotiable: Boolean, val condition: String, val category: Category, val seller: Seller,
    val latitude: Double?, val longitude: Double?, val address: String?, val city: String?, val state: String?,
    val status: String, val isFeatured: Boolean, val viewCount: Int, val images: List<String>,
    val createdAt: String, val updatedAt: String, val expiresAt: String?, val isFavorited: Boolean, val favoriteCount: Int)

data class PagedResult<T>(val data: List<T>, val total: Int, val page: Int, val pageSize: Int, val totalPages: Int)

// Chat
data class StartConversationRequest(val listingId: Long, val message: String)
data class SendMessageRequest(val text: String, val imageUrl: String?)
data class Conversation(val id: Long, val listingId: Long, val listingTitle: String, val listingThumbnail: String?,
    val listingPrice: Double, val otherUserId: Int, val otherUserName: String, val otherUserPhoto: String?,
    val lastMessage: String?, val lastMessageAt: String?, val unreadCount: Int)
data class ChatMessage(val id: Long, val senderId: Int, val senderName: String, val text: String, val imageUrl: String?, val isRead: Boolean, val sentAt: String)

// Favorite
data class FavoriteItem(val listingId: Long, val title: String, val price: Double, val currency: String,
    val thumbnailUrl: String?, val city: String?, val status: String, val favoritedAt: String)

// Saved Search
data class SaveSearchRequest(val query: String, val categoryId: Int?, val city: String?, val minPrice: Double?, val maxPrice: Double?, val notifyOnNew: Boolean)
data class SavedSearch(val id: Long, val query: String, val categoryId: Int?, val categoryName: String?,
    val city: String?, val minPrice: Double?, val maxPrice: Double?, val notifyOnNew: Boolean, val createdAt: String)

// Notification
data class NotificationItem(val id: Long, val type: String, val title: String, val body: String,
    val listingId: Long?, val conversationId: Long?, val isRead: Boolean, val createdAt: String)

// Report
data class ReportRequest(val reason: String, val description: String?)

// Generic
data class MessageResponse(val message: String)
data class IdResponse(val message: String, val id: Long)
data class UnreadCount(val count: Int)
