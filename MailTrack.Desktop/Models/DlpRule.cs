using System;

namespace MailTrack.Desktop.Models
{
    public class DlpRule
    {
        public long Id { get; set; }
        public string Name { get; set; } = "";
        public string Description { get; set; } = "";
        public string RuleType { get; set; } = "keyword"; // keyword, domain, regex, attachment_type, recipient_count, pii
        public string Pattern { get; set; } = "";
        public string Direction { get; set; } = "both"; // incoming, outgoing, both
        public string Severity { get; set; } = "medium"; // low, medium, high, critical
        public string Action { get; set; } = "alert"; // alert, flag, log
        public bool Enabled { get; set; } = true;
        public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
    }
}
