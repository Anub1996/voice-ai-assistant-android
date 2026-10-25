namespace WorkSentinel.Server.Models;

public class ProductivityScore
{
    public long Id { get; set; }
    public int EmployeeId { get; set; }
    public DateOnly Date { get; set; }
    public double Score { get; set; } // 0-100
    public int ActiveMinutes { get; set; }
    public int IdleMinutes { get; set; }
    public int ProductiveMinutes { get; set; }
    public int UnproductiveMinutes { get; set; }
    public int NeutralMinutes { get; set; }
    public int TotalAppsUsed { get; set; }
    public int TotalWebsitesVisited { get; set; }
    public int TotalEmailsSent { get; set; }
    public int TotalEmailsReceived { get; set; }
    public int AlertsTriggered { get; set; }

    // Navigation
    public Employee Employee { get; set; } = null!;
}
