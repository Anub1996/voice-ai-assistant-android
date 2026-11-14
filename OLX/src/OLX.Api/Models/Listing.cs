namespace OLX.Api.Models;

public class Listing
{
    public long Id { get; set; }
    public string Title { get; set; } = "";
    public string Slug { get; set; } = "";
    public string Description { get; set; } = "";
    public double Price { get; set; }
    public string Currency { get; set; } = "INR";
    public bool IsNegotiable { get; set; }
    public string Condition { get; set; } = "used"; // new, like_new, used, fair

    public int CategoryId { get; set; }
    public Category Category { get; set; } = null!;

    public int SellerId { get; set; }
    public User Seller { get; set; } = null!;

    public double? Latitude { get; set; }
    public double? Longitude { get; set; }
    public string? Address { get; set; }
    public string? City { get; set; }
    public string? State { get; set; }

    public string Status { get; set; } = "active"; // active, sold, expired, removed, under_review
    public bool IsFeatured { get; set; }
    public int ViewCount { get; set; }

    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
    public DateTime UpdatedAt { get; set; } = DateTime.UtcNow;
    public DateTime? ExpiresAt { get; set; }

    public ICollection<ListingImage> Images { get; set; } = new List<ListingImage>();
    public ICollection<Favorite> Favorites { get; set; } = new List<Favorite>();
    public ICollection<Conversation> Conversations { get; set; } = new List<Conversation>();
}

public class ListingImage
{
    public long Id { get; set; }
    public long ListingId { get; set; }
    public Listing Listing { get; set; } = null!;
    public string Url { get; set; } = "";
    public int SortOrder { get; set; }
}
