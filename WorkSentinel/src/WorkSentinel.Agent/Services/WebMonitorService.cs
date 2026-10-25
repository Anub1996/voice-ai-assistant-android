using Microsoft.Win32;

namespace WorkSentinel.Agent.Services;

public class WebMonitorService
{
    private readonly HashSet<string> _reportedUrls = new();
    private readonly List<WebVisitRecord> _records = new();
    private readonly object _lock = new();

    public void ScanBrowserHistory()
    {
        try
        {
            ScanChromeHistory();
            ScanEdgeHistory();
        }
        catch { }
    }

    private void ScanChromeHistory()
    {
        var historyPath = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "Google", "Chrome", "User Data", "Default", "History");
        ScanSqliteHistory(historyPath, "Chrome");
    }

    private void ScanEdgeHistory()
    {
        var historyPath = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
            "Microsoft", "Edge", "User Data", "Default", "History");
        ScanSqliteHistory(historyPath, "Edge");
    }

    private void ScanSqliteHistory(string dbPath, string browser)
    {
        if (!File.Exists(dbPath)) return;

        try
        {
            var tmpPath = Path.Combine(Path.GetTempPath(), $"ws_{browser}_history.db");
            File.Copy(dbPath, tmpPath, true);

            // Simple approach: read recent URLs from the file
            // In production, use Microsoft.Data.Sqlite
            var content = File.ReadAllText(tmpPath);
            // Placeholder - actual implementation would use SQLite to query urls table
        }
        catch { }
    }

    public void AddVisit(string url, string title, string browser)
    {
        var key = $"{url}_{DateTime.UtcNow:yyyyMMddHH}";
        lock (_lock)
        {
            if (_reportedUrls.Contains(key)) return;
            _reportedUrls.Add(key);

            var uri = new Uri(url);
            _records.Add(new WebVisitRecord
            {
                Url = url,
                Title = title,
                Domain = uri.Host,
                Browser = browser,
                VisitTime = DateTime.UtcNow,
                DurationSeconds = 0
            });
        }
    }

    public List<WebVisitRecord> Flush()
    {
        lock (_lock)
        {
            var result = new List<WebVisitRecord>(_records);
            _records.Clear();
            return result;
        }
    }
}

public class WebVisitRecord
{
    public string Url { get; set; } = "";
    public string Title { get; set; } = "";
    public string Domain { get; set; } = "";
    public string Browser { get; set; } = "";
    public DateTime VisitTime { get; set; }
    public int DurationSeconds { get; set; }
}
