using System;

namespace MailTrack.Desktop.Models
{
    public class TrackedEmail
    {
        public long Id { get; set; }
        public string Sender { get; set; } = "";
        public string Recipients { get; set; } = "";
        public string Cc { get; set; } = "";
        public string Bcc { get; set; } = "";
        public string Subject { get; set; } = "";
        public string BodyPreview { get; set; } = "";
        public string Direction { get; set; } = "incoming"; // "incoming" or "outgoing"
        public string EmailClient { get; set; } = ""; // "Outlook", "IMAP", etc.
        public string Account { get; set; } = "";
        public bool HasAttachment { get; set; }
        public string AttachmentNames { get; set; } = "";
        public long AttachmentSizeBytes { get; set; }
        public DateTime Timestamp { get; set; } = DateTime.UtcNow;
        public bool Flagged { get; set; }
        public bool AlertTriggered { get; set; }
        public long? AlertRuleId { get; set; }
    }
}
