namespace MailTrack.Desktop.Models
{
    public class MonitoredAccount
    {
        public long Id { get; set; }
        public string Name { get; set; } = "";
        public string AccountType { get; set; } = "imap"; // imap, outlook_com
        public string Host { get; set; } = "";
        public int Port { get; set; } = 993;
        public string Email { get; set; } = "";
        public string Password { get; set; } = "";
        public bool UseSsl { get; set; } = true;
        public bool Enabled { get; set; } = true;
        public int PollIntervalSeconds { get; set; } = 60;
    }
}
