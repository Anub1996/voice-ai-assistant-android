namespace OLX.Api.Models;

public class Notification
{
    public long Id { get; set; }
    public int UserId { get; set; }
    public User User { get; set; } = null!;
    public string Type { get; set; } = ""; // message, price_drop, new_listing, favorite_sold, listing_expired, listing_approved
    public string Title { get; set; } = "";
    public string Body { get; set; } = "";
    public long? ListingId { get; set; }
    public long? ConversationId { get; set; }
    public bool IsRead { get; set; }
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
}

public class Report
{
    public long Id { get; set; }
    public int ReporterId { get; set; }
    public User Reporter { get; set; } = null!;
    public long ListingId { get; set; }
    public Listing Listing { get; set; } = null!;
    public string Reason { get; set; } = ""; // spam, fraud, prohibited, duplicate, wrong_category, other
    public string? Description { get; set; }
    public string Status { get; set; } = "pending"; // pending, reviewed, resolved
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
}
