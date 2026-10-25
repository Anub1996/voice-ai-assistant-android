namespace WorkSentinel.Agent.Config;

public class AgentConfig
{
    public string ServerUrl { get; set; } = "https://localhost:5001";
    public string ComputerName { get; set; } = Environment.MachineName;
    public string EmployeeId { get; set; } = "";
    public string EmployeeName { get; set; } = "";
    public string Email { get; set; } = "";
    public string Department { get; set; } = "";
    public int HeartbeatIntervalSeconds { get; set; } = 30;
    public int DataSyncIntervalSeconds { get; set; } = 60;
    public int ScreenshotIntervalSeconds { get; set; } = 300;
    public int IdleThresholdSeconds { get; set; } = 180;
    public bool EnableScreenshots { get; set; } = true;
    public bool EnableKeystrokeCount { get; set; } = true;
    public bool EnableWebMonitoring { get; set; } = true;
    public int ScreenshotQuality { get; set; } = 50;
}
