namespace OLX.Api.Models;

public class Favorite
{
    public long Id { get; set; }
    public int UserId { get; set; }
    public User User { get; set; } = null!;
    public long ListingId { get; set; }
    public Listing Listing { get; set; } = null!;
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
}

public class SavedSearch
{
    public long Id { get; set; }
    public int UserId { get; set; }
    public User User { get; set; } = null!;
    public string Query { get; set; } = "";
    public int? CategoryId { get; set; }
    public string? City { get; set; }
    public double? MinPrice { get; set; }
    public double? MaxPrice { get; set; }
    public bool NotifyOnNew { get; set; } = true;
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
}
