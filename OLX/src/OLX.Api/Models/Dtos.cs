namespace OLX.Api.Models;

// ── Auth ──
public record RegisterRequest(string Username, string Email, string Phone, string Password, string FullName);
public record LoginRequest(string Username, string Password);
public record LoginResponse(string Token, int UserId, string Username, string FullName, string? ProfilePhotoUrl, string ExpiresAt);
public record DeviceTokenRequest(string Token);
public record ChangePasswordRequest(string OldPassword, string NewPassword);

// ── User / Profile ──
public record UserProfileDto(int Id, string Username, string FullName, string Email, string Phone, string? ProfilePhotoUrl,
    string? Bio, string? City, string? State, DateTime JoinedAt, bool IsVerified, int ListingsCount, double AvgResponseTime);
public record UpdateProfileRequest(string? FullName, string? Phone, string? Bio, string? City, string? State);
public record SellerDto(int Id, string Username, string FullName, string? ProfilePhotoUrl, string? City, DateTime JoinedAt, bool IsVerified);

// ── Category ──
public record CategoryDto(int Id, string Name, string Slug, string Icon, int? ParentId, List<CategoryDto>? Children);

// ── Listing ──
public record CreateListingRequest(string Title, string Description, double Price, string? Currency, bool IsNegotiable,
    string Condition, int CategoryId, double? Latitude, double? Longitude, string? Address, string? City, string? State,
    List<string>? ImageUrls);
public record UpdateListingRequest(string? Title, string? Description, double? Price, bool? IsNegotiable,
    string? Condition, int? CategoryId, string? Address, string? City, string? State);

public record ListingListDto(long Id, string Title, string Slug, double Price, string Currency, bool IsNegotiable,
    string Condition, string? City, string? State, string Status, bool IsFeatured, int ViewCount,
    string? ThumbnailUrl, string CategoryName, string SellerName, int SellerId, DateTime CreatedAt, bool IsFavorited);
public record ListingDetailDto(long Id, string Title, string Slug, string Description, double Price, string Currency,
    bool IsNegotiable, string Condition, CategoryDto Category, SellerDto Seller, double? Latitude, double? Longitude,
    string? Address, string? City, string? State, string Status, bool IsFeatured, int ViewCount,
    List<string> Images, DateTime CreatedAt, DateTime UpdatedAt, DateTime? ExpiresAt, bool IsFavorited, int FavoriteCount);

public record ListingSearchRequest(string? Query, int? CategoryId, string? City, string? State, double? MinPrice,
    double? MaxPrice, string? Condition, string? SortBy, int Page = 1, int PageSize = 20);

// ── Chat ──
public record StartConversationRequest(long ListingId, string Message);
public record SendMessageRequest(string Text, string? ImageUrl);
public record ConversationDto(long Id, long ListingId, string ListingTitle, string? ListingThumbnail, double ListingPrice,
    int OtherUserId, string OtherUserName, string? OtherUserPhoto, string? LastMessage, DateTime? LastMessageAt, int UnreadCount);
public record MessageDto(long Id, int SenderId, string SenderName, string Text, string? ImageUrl, bool IsRead, DateTime SentAt);

// ── Favorite ──
public record FavoriteDto(long ListingId, string Title, double Price, string Currency, string? ThumbnailUrl,
    string? City, string Status, DateTime FavoritedAt);

// ── Saved Search ──
public record SaveSearchRequest(string Query, int? CategoryId, string? City, double? MinPrice, double? MaxPrice, bool NotifyOnNew);
public record SavedSearchDto(long Id, string Query, int? CategoryId, string? CategoryName, string? City,
    double? MinPrice, double? MaxPrice, bool NotifyOnNew, DateTime CreatedAt);

// ── Notification ──
public record NotificationDto(long Id, string Type, string Title, string Body, long? ListingId,
    long? ConversationId, bool IsRead, DateTime CreatedAt);

// ── Report ──
public record ReportRequest(string Reason, string? Description);

// ── Pagination ──
public record PagedResult<T>(List<T> Data, int Total, int Page, int PageSize, int TotalPages);

// ── Generic ──
public record MessageResponse(string Message);
public record IdResponse(string Message, long Id);
