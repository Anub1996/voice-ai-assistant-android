using System;

namespace MailTrack.Desktop.Models
{
    public class Alert
    {
        public long Id { get; set; }
        public long EmailId { get; set; }
        public long RuleId { get; set; }
        public string RuleName { get; set; } = "";
        public string Severity { get; set; } = "";
        public string Description { get; set; } = "";
        public bool Acknowledged { get; set; }
        public DateTime Timestamp { get; set; } = DateTime.UtcNow;
    }
}
