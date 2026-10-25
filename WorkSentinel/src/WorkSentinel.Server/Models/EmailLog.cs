using System.ComponentModel.DataAnnotations;

namespace WorkSentinel.Server.Models;

public class EmailLog
{
    public long Id { get; set; }
    public int EmployeeId { get; set; }

    [MaxLength(500)]
    public string Sender { get; set; } = "";

    [MaxLength(2000)]
    public string Recipients { get; set; } = "";

    [MaxLength(500)]
    public string Cc { get; set; } = "";

    [MaxLength(500)]
    public string Subject { get; set; } = "";

    [MaxLength(2000)]
    public string BodyPreview { get; set; } = "";

    [MaxLength(50)]
    public string Direction { get; set; } = "incoming"; // incoming, outgoing

    [MaxLength(100)]
    public string EmailClient { get; set; } = "";

    public bool HasAttachment { get; set; }

    [MaxLength(500)]
    public string AttachmentNames { get; set; } = "";

    public bool Flagged { get; set; }
    public bool AlertTriggered { get; set; }
    public DateTime Timestamp { get; set; }

    // Navigation
    public Employee Employee { get; set; } = null!;
}
