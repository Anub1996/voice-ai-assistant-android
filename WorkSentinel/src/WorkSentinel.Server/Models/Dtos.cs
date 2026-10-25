namespace WorkSentinel.Server.Models;

// Auth
public record LoginRequest(string Username, string Password);
public record LoginResponse(string Token, string Username, string Role, DateTime ExpiresAt);

// Agent registration
public record AgentRegisterRequest(string EmployeeId, string FullName, string Email, string Department, string ComputerName);
public record AgentHeartbeatRequest(string EmployeeId, string Status, string ActiveWindow, string ActiveApp);

// Agent data ingestion (batch)
public record AgentDataPayload
{
    public string EmployeeId { get; init; } = "";
    public List<ActivityBatch> Activities { get; init; } = new();
    public List<AppUsageBatch> AppUsages { get; init; } = new();
    public List<WebVisitBatch> WebVisits { get; init; } = new();
    public List<EmailBatch> Emails { get; init; } = new();
    public List<FileBatch> Files { get; init; } = new();
}

public record ActivityBatch(string ActivityType, DateTime StartTime, DateTime? EndTime, int DurationSeconds, string ActiveWindow, string ActiveApp, int KeystrokeCount, int MouseClickCount);
public record AppUsageBatch(string AppName, string WindowTitle, string ProcessName, DateTime StartTime, int DurationSeconds, bool IsFocused);
public record WebVisitBatch(string Url, string Title, string Domain, string Browser, DateTime VisitTime, int DurationSeconds);
public record EmailBatch(string Sender, string Recipients, string Cc, string Subject, string BodyPreview, string Direction, string EmailClient, bool HasAttachment, string AttachmentNames, DateTime Timestamp);
public record FileBatch(string FilePath, string FileName, string Action, string Destination, long FileSizeBytes, DateTime Timestamp);

// Dashboard
public record DashboardStats
{
    public int TotalEmployees { get; init; }
    public int OnlineEmployees { get; init; }
    public int IdleEmployees { get; init; }
    public double AvgProductivityScore { get; init; }
    public int TotalAlertsToday { get; init; }
    public int UnacknowledgedAlerts { get; init; }
    public int TotalEmailsToday { get; init; }
    public int TotalWebVisitsToday { get; init; }
    public List<TopAppDto> TopApps { get; init; } = new();
    public List<TopWebDto> TopWebsites { get; init; } = new();
    public List<AlertDto> RecentAlerts { get; init; } = new();
    public List<ProductivityTrend> ProductivityTrend { get; init; } = new();
}

public record TopAppDto(string AppName, string Category, int TotalMinutes, int EmployeeCount);
public record TopWebDto(string Domain, string Category, int TotalMinutes, int EmployeeCount);
public record AlertDto(long Id, string EmployeeName, string Category, string Severity, string Title, string Description, bool Acknowledged, DateTime Timestamp);
public record ProductivityTrend(string Date, double Score, int ActiveMinutes, int IdleMinutes);

// Employee detail
public record EmployeeDetailDto
{
    public int Id { get; init; }
    public string FullName { get; init; } = "";
    public string Email { get; init; } = "";
    public string Department { get; init; } = "";
    public string ComputerName { get; init; } = "";
    public string Status { get; init; } = "";
    public DateTime? LastSeenAt { get; init; }
    public double TodayProductivity { get; init; }
    public int ActiveMinutesToday { get; init; }
    public int IdleMinutesToday { get; init; }
    public int AlertsToday { get; init; }
    public List<TimelineEntry> Timeline { get; init; } = new();
}

public record TimelineEntry(DateTime Time, string Type, string Description, string Detail);

// Report
public record ProductivityReport
{
    public string Period { get; init; } = "";
    public List<EmployeeProductivityRow> Rows { get; init; } = new();
}

public record EmployeeProductivityRow
{
    public string EmployeeName { get; init; } = "";
    public string Department { get; init; } = "";
    public double AvgScore { get; init; }
    public int TotalActiveHours { get; init; }
    public int TotalIdleHours { get; init; }
    public string TopApp { get; init; } = "";
    public string TopWebsite { get; init; } = "";
    public int Alerts { get; init; }
}
