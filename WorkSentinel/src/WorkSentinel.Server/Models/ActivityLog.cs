using System.ComponentModel.DataAnnotations;

namespace WorkSentinel.Server.Models;

public class ActivityLog
{
    public long Id { get; set; }
    public int EmployeeId { get; set; }

    [MaxLength(50)]
    public string ActivityType { get; set; } = ""; // active, idle, locked, break

    public DateTime StartTime { get; set; }
    public DateTime? EndTime { get; set; }
    public int DurationSeconds { get; set; }

    [MaxLength(200)]
    public string ActiveWindow { get; set; } = "";

    [MaxLength(200)]
    public string ActiveApp { get; set; } = "";

    public int KeystrokeCount { get; set; }
    public int MouseClickCount { get; set; }

    // Navigation
    public Employee Employee { get; set; } = null!;
}
