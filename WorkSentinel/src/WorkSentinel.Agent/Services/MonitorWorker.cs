using WorkSentinel.Agent.Config;

namespace WorkSentinel.Agent.Services;

public class MonitorWorker : BackgroundService
{
    private readonly AgentApiClient _api;
    private readonly AgentConfig _config;
    private readonly AppMonitorService _appMonitor;
    private readonly IdleDetector _idle;
    private readonly ScreenshotService _screenshot;
    private readonly KeystrokeService _keystroke;
    private readonly WebMonitorService _webMonitor;
    private readonly ILogger<MonitorWorker> _logger;

    public MonitorWorker(
        AgentApiClient api,
        AgentConfig config,
        AppMonitorService appMonitor,
        IdleDetector idle,
        ScreenshotService screenshot,
        KeystrokeService keystroke,
        WebMonitorService webMonitor,
        ILogger<MonitorWorker> logger)
    {
        _api = api;
        _config = config;
        _appMonitor = appMonitor;
        _idle = idle;
        _screenshot = screenshot;
        _keystroke = keystroke;
        _webMonitor = webMonitor;
        _logger = logger;
    }

    protected override async Task ExecuteAsync(CancellationToken ct)
    {
        _logger.LogInformation("WorkSentinel Agent starting on {Computer}", _config.ComputerName);

        while (!await _api.RegisterAsync())
        {
            _logger.LogWarning("Registration failed, retrying in 10s");
            await Task.Delay(10_000, ct);
        }
        _logger.LogInformation("Agent registered successfully");

        var heartbeatTask = RunHeartbeat(ct);
        var monitorTask = RunMonitor(ct);
        var syncTask = RunDataSync(ct);
        var screenshotTask = RunScreenshots(ct);

        await Task.WhenAll(heartbeatTask, monitorTask, syncTask, screenshotTask);
    }

    private async Task RunHeartbeat(CancellationToken ct)
    {
        while (!ct.IsCancellationRequested)
        {
            try
            {
                var status = _idle.GetStatus();
                var (app, window) = _appMonitor.GetActiveWindow();
                await _api.SendHeartbeatAsync(status, window, app);
            }
            catch (Exception ex) { _logger.LogWarning(ex, "Heartbeat error"); }

            await Task.Delay(_config.HeartbeatIntervalSeconds * 1000, ct);
        }
    }

    private async Task RunMonitor(CancellationToken ct)
    {
        while (!ct.IsCancellationRequested)
        {
            try
            {
                _appMonitor.Track();
                _keystroke.Poll();
                if (_config.EnableWebMonitoring) _webMonitor.ScanBrowserHistory();
            }
            catch (Exception ex) { _logger.LogWarning(ex, "Monitor error"); }

            await Task.Delay(5000, ct);
        }
    }

    private async Task RunDataSync(CancellationToken ct)
    {
        while (!ct.IsCancellationRequested)
        {
            await Task.Delay(_config.DataSyncIntervalSeconds * 1000, ct);

            try
            {
                var apps = _appMonitor.Flush();
                var activities = _idle.Flush();
                var webVisits = _webMonitor.Flush();
                var (keystrokes, clicks) = _keystroke.Flush();

                foreach (var a in activities)
                {
                    a.KeystrokeCount = keystrokes;
                    a.MouseClickCount = clicks;
                    a.ActiveWindow = _appMonitor.GetActiveWindow().window;
                    a.ActiveApp = _appMonitor.GetActiveWindow().app;
                }

                if (activities.Count == 0)
                {
                    activities.Add(new ActivityRecord
                    {
                        ActivityType = _idle.IsIdle ? "idle" : "active",
                        StartTime = DateTime.UtcNow.AddSeconds(-_config.DataSyncIntervalSeconds),
                        EndTime = DateTime.UtcNow,
                        DurationSeconds = _config.DataSyncIntervalSeconds,
                        ActiveWindow = _appMonitor.GetActiveWindow().window,
                        ActiveApp = _appMonitor.GetActiveWindow().app,
                        KeystrokeCount = keystrokes,
                        MouseClickCount = clicks
                    });
                }

                var payload = new
                {
                    computerName = _config.ComputerName,
                    activities = activities.Select(a => new
                    {
                        a.ActivityType, a.StartTime, a.EndTime, a.DurationSeconds,
                        a.ActiveWindow, a.ActiveApp, a.KeystrokeCount, a.MouseClickCount
                    }),
                    appUsages = apps.Select(a => new
                    {
                        a.AppName, a.WindowTitle, a.ProcessName, a.StartTime,
                        a.DurationSeconds, a.IsFocused
                    }),
                    webVisits = webVisits.Select(w => new
                    {
                        w.Url, w.Title, w.Domain, w.Browser,
                        w.VisitTime, w.DurationSeconds
                    }),
                    emails = Array.Empty<object>(),
                    fileActivities = Array.Empty<object>()
                };

                await _api.SendDataAsync(payload);
                _logger.LogDebug("Data synced: {Apps} apps, {Activities} activities, {Web} web visits",
                    apps.Count, activities.Count, webVisits.Count);
            }
            catch (Exception ex)
            {
                _logger.LogError(ex, "Data sync error");
            }
        }
    }

    private async Task RunScreenshots(CancellationToken ct)
    {
        if (!_config.EnableScreenshots) return;

        while (!ct.IsCancellationRequested)
        {
            await Task.Delay(_config.ScreenshotIntervalSeconds * 1000, ct);

            try
            {
                var data = _screenshot.CaptureScreen();
                if (data != null)
                {
                    await _api.UploadScreenshotAsync(_config.ComputerName, data);
                    _logger.LogDebug("Screenshot uploaded ({Size} KB)", data.Length / 1024);
                }
            }
            catch (Exception ex) { _logger.LogWarning(ex, "Screenshot error"); }
        }
    }
}
