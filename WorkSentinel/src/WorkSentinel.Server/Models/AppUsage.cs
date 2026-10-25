using System.ComponentModel.DataAnnotations;

namespace WorkSentinel.Server.Models;

public class AppUsage
{
    public long Id { get; set; }
    public int EmployeeId { get; set; }

    [Required, MaxLength(200)]
    public string AppName { get; set; } = "";

    [MaxLength(500)]
    public string WindowTitle { get; set; } = "";

    [MaxLength(200)]
    public string ProcessName { get; set; } = "";

    [MaxLength(50)]
    public string Category { get; set; } = "uncategorized"; // productive, unproductive, neutral, uncategorized

    public DateTime StartTime { get; set; }
    public DateTime? EndTime { get; set; }
    public int DurationSeconds { get; set; }
    public bool IsFocused { get; set; }

    // Navigation
    public Employee Employee { get; set; } = null!;
}
