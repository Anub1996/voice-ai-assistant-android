using System.ComponentModel.DataAnnotations;

namespace WorkSentinel.Server.Models;

public class Employee
{
    public int Id { get; set; }

    [Required, MaxLength(100)]
    public string EmployeeId { get; set; } = "";

    [Required, MaxLength(200)]
    public string FullName { get; set; } = "";

    [MaxLength(200)]
    public string Email { get; set; } = "";

    [MaxLength(100)]
    public string Department { get; set; } = "";

    [MaxLength(100)]
    public string Designation { get; set; } = "";

    [MaxLength(100)]
    public string ComputerName { get; set; } = "";

    [MaxLength(50)]
    public string Status { get; set; } = "offline"; // online, idle, offline

    public bool MonitoringEnabled { get; set; } = true;
    public DateTime? LastSeenAt { get; set; }
    public DateTime RegisteredAt { get; set; } = DateTime.UtcNow;

    // Navigation
    public ICollection<ActivityLog> Activities { get; set; } = new List<ActivityLog>();
    public ICollection<AppUsage> AppUsages { get; set; } = new List<AppUsage>();
    public ICollection<WebVisit> WebVisits { get; set; } = new List<WebVisit>();
    public ICollection<ScreenCapture> ScreenCaptures { get; set; } = new List<ScreenCapture>();
    public ICollection<EmailLog> EmailLogs { get; set; } = new List<EmailLog>();
    public ICollection<FileActivity> FileActivities { get; set; } = new List<FileActivity>();
    public ICollection<Alert> Alerts { get; set; } = new List<Alert>();
}
