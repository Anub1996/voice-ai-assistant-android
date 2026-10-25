using System.ComponentModel.DataAnnotations;

namespace WorkSentinel.Server.Models;

public class AdminUser
{
    public int Id { get; set; }

    [Required, MaxLength(100)]
    public string Username { get; set; } = "";

    [Required]
    public string PasswordHash { get; set; } = "";

    [MaxLength(200)]
    public string FullName { get; set; } = "";

    [MaxLength(200)]
    public string Email { get; set; } = "";

    [MaxLength(50)]
    public string Role { get; set; } = "admin"; // admin, manager, viewer

    public bool IsActive { get; set; } = true;
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
    public DateTime? LastLoginAt { get; set; }
}
