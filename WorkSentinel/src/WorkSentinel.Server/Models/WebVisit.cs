using System.ComponentModel.DataAnnotations;

namespace WorkSentinel.Server.Models;

public class WebVisit
{
    public long Id { get; set; }
    public int EmployeeId { get; set; }

    [Required, MaxLength(2000)]
    public string Url { get; set; } = "";

    [MaxLength(500)]
    public string Title { get; set; } = "";

    [MaxLength(200)]
    public string Domain { get; set; } = "";

    [MaxLength(50)]
    public string Category { get; set; } = "uncategorized"; // productive, unproductive, social, news, entertainment, uncategorized

    [MaxLength(50)]
    public string Browser { get; set; } = "";

    public DateTime VisitTime { get; set; }
    public int DurationSeconds { get; set; }

    // Navigation
    public Employee Employee { get; set; } = null!;
}
