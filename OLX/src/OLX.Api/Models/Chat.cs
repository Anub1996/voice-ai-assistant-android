namespace OLX.Api.Models;

public class Conversation
{
    public long Id { get; set; }
    public long ListingId { get; set; }
    public Listing Listing { get; set; } = null!;
    public int BuyerId { get; set; }
    public User Buyer { get; set; } = null!;
    public int SellerId { get; set; }
    public User SellerUser { get; set; } = null!;
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
    public DateTime? LastMessageAt { get; set; }

    public ICollection<Message> Messages { get; set; } = new List<Message>();
}

public class Message
{
    public long Id { get; set; }
    public long ConversationId { get; set; }
    public Conversation Conversation { get; set; } = null!;
    public int SenderId { get; set; }
    public User Sender { get; set; } = null!;
    public string Text { get; set; } = "";
    public string? ImageUrl { get; set; }
    public bool IsRead { get; set; }
    public DateTime SentAt { get; set; } = DateTime.UtcNow;
}
