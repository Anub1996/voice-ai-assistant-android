using System.ComponentModel.DataAnnotations;

namespace WorkSentinel.Server.Models;

public class Alert
{
    public long Id { get; set; }
    public int EmployeeId { get; set; }
    public int? PolicyId { get; set; }

    [Required, MaxLength(50)]
    public string Category { get; set; } = ""; // dlp, productivity, anomaly, policy_violation

    [Required, MaxLength(50)]
    public string Severity { get; set; } = "medium";

    [Required, MaxLength(500)]
    public string Title { get; set; } = "";

    [MaxLength(2000)]
    public string Description { get; set; } = "";

    [MaxLength(2000)]
    public string Details { get; set; } = "";

    public bool Acknowledged { get; set; }
    public DateTime? AcknowledgedAt { get; set; }

    [MaxLength(200)]
    public string AcknowledgedBy { get; set; } = "";

    public DateTime Timestamp { get; set; } = DateTime.UtcNow;

    // Navigation
    public Employee Employee { get; set; } = null!;
    public DlpPolicy? Policy { get; set; }
}
