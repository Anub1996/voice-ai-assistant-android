using System.ComponentModel.DataAnnotations;

namespace ZingHR.Api.Models;

public class Notification
{
    public long Id { get; set; }
    public int EmployeeId { get; set; }

    [Required, MaxLength(300)]
    public string Title { get; set; } = "";

    [MaxLength(1000)]
    public string Body { get; set; } = "";

    [MaxLength(50)]
    public string Type { get; set; } = "info"; // info, leave, attendance, payslip, expense, announcement, helpdesk

    [MaxLength(200)]
    public string? ActionUrl { get; set; }

    public long? ReferenceId { get; set; }

    public bool IsRead { get; set; }
    public DateTime? ReadAt { get; set; }
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;

    public Employee Employee { get; set; } = null!;
}

public class Announcement
{
    public int Id { get; set; }

    [Required, MaxLength(300)]
    public string Title { get; set; } = "";

    [MaxLength(5000)]
    public string Content { get; set; } = "";

    [MaxLength(50)]
    public string Category { get; set; } = "general"; // general, policy, event, celebration, urgent

    [MaxLength(50)]
    public string Priority { get; set; } = "normal"; // low, normal, high, urgent

    public bool IsPublished { get; set; }
    public int CreatedBy { get; set; }
    public DateTime PublishedAt { get; set; } = DateTime.UtcNow;
    public DateTime? ExpiresAt { get; set; }
}

public class Holiday
{
    public int Id { get; set; }

    [Required, MaxLength(200)]
    public string Name { get; set; } = "";

    public DateOnly Date { get; set; }

    [MaxLength(50)]
    public string Type { get; set; } = "public"; // public, restricted, optional

    public int Year { get; set; }
    public bool IsActive { get; set; } = true;
}
