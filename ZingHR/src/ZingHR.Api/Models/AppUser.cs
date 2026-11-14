using System.ComponentModel.DataAnnotations;

namespace ZingHR.Api.Models;

public class AppUser
{
    public int Id { get; set; }

    [Required, MaxLength(100)]
    public string Username { get; set; } = "";

    [Required]
    public string PasswordHash { get; set; } = "";

    [Required, MaxLength(50)]
    public string Role { get; set; } = "employee"; // admin, hr, manager, employee

    public int? EmployeeId { get; set; }

    [MaxLength(200)]
    public string FullName { get; set; } = "";

    [MaxLength(200)]
    public string Email { get; set; } = "";

    public bool IsActive { get; set; } = true;
    public DateTime? LastLoginAt { get; set; }
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;

    [MaxLength(6)]
    public string? QuickPin { get; set; }

    [MaxLength(500)]
    public string? DeviceToken { get; set; }

    public Employee? Employee { get; set; }
}
