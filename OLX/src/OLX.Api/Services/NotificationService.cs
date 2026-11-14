using OLX.Api.Data;
using OLX.Api.Models;

namespace OLX.Api.Services;

public class NotificationService
{
    private readonly AppDbContext _db;

    public NotificationService(AppDbContext db) => _db = db;

    public async Task CreateAsync(int userId, string type, string title, string body, long? listingId = null, long? conversationId = null)
    {
        _db.Notifications.Add(new Notification
        {
            UserId = userId,
            Type = type,
            Title = title,
            Body = body,
            ListingId = listingId,
            ConversationId = conversationId
        });
        await _db.SaveChangesAsync();
    }
}
