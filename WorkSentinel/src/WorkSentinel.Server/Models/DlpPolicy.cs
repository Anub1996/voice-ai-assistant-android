using System.ComponentModel.DataAnnotations;

namespace WorkSentinel.Server.Models;

public class DlpPolicy
{
    public int Id { get; set; }

    [Required, MaxLength(200)]
    public string Name { get; set; } = "";

    [MaxLength(500)]
    public string Description { get; set; } = "";

    [Required, MaxLength(50)]
    public string Category { get; set; } = "email"; // email, web, app, file, data

    [Required, MaxLength(50)]
    public string RuleType { get; set; } = "keyword"; // keyword, regex, domain, file_type, recipient_count, pii, app_block, web_block

    [Required, MaxLength(1000)]
    public string Pattern { get; set; } = "";

    [MaxLength(50)]
    public string Severity { get; set; } = "medium"; // low, medium, high, critical

    [MaxLength(50)]
    public string Action { get; set; } = "alert"; // alert, flag, block, log

    public bool Enabled { get; set; } = true;
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
    public DateTime? UpdatedAt { get; set; }

    // Navigation
    public ICollection<Alert> Alerts { get; set; } = new List<Alert>();
}
