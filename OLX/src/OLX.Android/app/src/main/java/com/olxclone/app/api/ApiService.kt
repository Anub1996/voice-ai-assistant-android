package com.olxclone.app.api

import retrofit2.Response
import retrofit2.http.*

interface ApiService {
    // Auth
    @POST("api/auth/register") suspend fun register(@Body req: RegisterRequest): Response<LoginResponse>
    @POST("api/auth/login") suspend fun login(@Body req: LoginRequest): Response<LoginResponse>
    @POST("api/auth/device-token") suspend fun setDeviceToken(@Body req: DeviceTokenRequest): Response<MessageResponse>
    @POST("api/auth/change-password") suspend fun changePassword(@Body req: ChangePasswordRequest): Response<MessageResponse>

    // Users
    @GET("api/users/me") suspend fun getProfile(): Response<UserProfile>
    @PUT("api/users/me") suspend fun updateProfile(@Body req: UpdateProfileRequest): Response<MessageResponse>
    @GET("api/users/{id}") suspend fun getSeller(@Path("id") id: Int): Response<Seller>
    @GET("api/users/{id}/listings") suspend fun getSellerListings(@Path("id") id: Int, @Query("page") page: Int = 1): Response<PagedResult<ListingItem>>

    // Categories
    @GET("api/categories") suspend fun getCategories(): Response<List<Category>>
    @GET("api/categories/{id}") suspend fun getCategory(@Path("id") id: Int): Response<Category>
    @GET("api/categories/{id}/listings") suspend fun getCategoryListings(@Path("id") id: Int, @Query("page") page: Int = 1): Response<PagedResult<ListingItem>>

    // Listings
    @GET("api/listings") suspend fun getListings(
        @Query("query") query: String? = null, @Query("categoryId") categoryId: Int? = null,
        @Query("city") city: String? = null, @Query("state") state: String? = null,
        @Query("minPrice") minPrice: Double? = null, @Query("maxPrice") maxPrice: Double? = null,
        @Query("condition") condition: String? = null, @Query("sortBy") sortBy: String? = null,
        @Query("page") page: Int = 1, @Query("pageSize") pageSize: Int = 20): Response<PagedResult<ListingItem>>
    @GET("api/listings/featured") suspend fun getFeaturedListings(): Response<List<ListingItem>>
    @GET("api/listings/{id}") suspend fun getListingDetail(@Path("id") id: Long): Response<ListingDetail>
    @GET("api/listings/my") suspend fun getMyListings(): Response<List<ListingItem>>
    @POST("api/listings") suspend fun createListing(@Body req: CreateListingRequest): Response<IdResponse>
    @PUT("api/listings/{id}") suspend fun updateListing(@Path("id") id: Long, @Body req: UpdateListingRequest): Response<MessageResponse>
    @PUT("api/listings/{id}/status") suspend fun updateListingStatus(@Path("id") id: Long, @Body status: String): Response<MessageResponse>
    @DELETE("api/listings/{id}") suspend fun deleteListing(@Path("id") id: Long): Response<MessageResponse>

    // Chat
    @GET("api/chat/conversations") suspend fun getConversations(): Response<List<Conversation>>
    @GET("api/chat/conversations/{id}/messages") suspend fun getMessages(@Path("id") id: Long): Response<List<ChatMessage>>
    @POST("api/chat/conversations") suspend fun startConversation(@Body req: StartConversationRequest): Response<IdResponse>
    @POST("api/chat/conversations/{id}/messages") suspend fun sendMessage(@Path("id") id: Long, @Body req: SendMessageRequest): Response<ChatMessage>

    // Favorites
    @GET("api/favorites") suspend fun getFavorites(): Response<List<FavoriteItem>>
    @POST("api/favorites/{listingId}") suspend fun addFavorite(@Path("listingId") id: Long): Response<MessageResponse>
    @DELETE("api/favorites/{listingId}") suspend fun removeFavorite(@Path("listingId") id: Long): Response<MessageResponse>
    @GET("api/favorites/saved-searches") suspend fun getSavedSearches(): Response<List<SavedSearch>>
    @POST("api/favorites/saved-searches") suspend fun saveSearch(@Body req: SaveSearchRequest): Response<IdResponse>
    @DELETE("api/favorites/saved-searches/{id}") suspend fun deleteSavedSearch(@Path("id") id: Long): Response<MessageResponse>

    // Notifications
    @GET("api/notifications") suspend fun getNotifications(): Response<List<NotificationItem>>
    @GET("api/notifications/unread-count") suspend fun getUnreadCount(): Response<UnreadCount>
    @PUT("api/notifications/{id}/read") suspend fun markRead(@Path("id") id: Long): Response<MessageResponse>
    @PUT("api/notifications/read-all") suspend fun markAllRead(): Response<MessageResponse>

    // Reports
    @POST("api/reports/listings/{listingId}") suspend fun reportListing(@Path("listingId") id: Long, @Body req: ReportRequest): Response<MessageResponse>
}
