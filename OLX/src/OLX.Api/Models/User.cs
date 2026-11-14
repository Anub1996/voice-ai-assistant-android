namespace OLX.Api.Models;

public class User
{
    public int Id { get; set; }
    public string Username { get; set; } = "";
    public string Email { get; set; } = "";
    public string Phone { get; set; } = "";
    public string PasswordHash { get; set; } = "";
    public string FullName { get; set; } = "";
    public string? ProfilePhotoUrl { get; set; }
    public string? Bio { get; set; }
    public string? City { get; set; }
    public string? State { get; set; }
    public DateTime JoinedAt { get; set; } = DateTime.UtcNow;
    public DateTime? LastActiveAt { get; set; }
    public bool IsVerified { get; set; }
    public string? DeviceToken { get; set; }

    public ICollection<Listing> Listings { get; set; } = new List<Listing>();
    public ICollection<Favorite> Favorites { get; set; } = new List<Favorite>();
    public ICollection<Notification> Notifications { get; set; } = new List<Notification>();
}
