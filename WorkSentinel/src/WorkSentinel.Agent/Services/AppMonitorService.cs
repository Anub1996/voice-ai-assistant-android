using System.Diagnostics;
using System.Runtime.InteropServices;

namespace WorkSentinel.Agent.Services;

public class AppMonitorService
{
    [DllImport("user32.dll")]
    private static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll", SetLastError = true)]
    private static extern int GetWindowText(IntPtr hWnd, System.Text.StringBuilder text, int count);

    [DllImport("user32.dll", SetLastError = true)]
    private static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);

    private string _currentApp = "";
    private string _currentWindow = "";
    private DateTime _appStartTime = DateTime.UtcNow;
    private readonly List<AppUsageRecord> _records = new();
    private readonly object _lock = new();

    public (string app, string window) GetActiveWindow()
    {
        try
        {
            var hwnd = GetForegroundWindow();
            var sb = new System.Text.StringBuilder(256);
            GetWindowText(hwnd, sb, 256);
            var windowTitle = sb.ToString();

            GetWindowThreadProcessId(hwnd, out uint pid);
            var proc = Process.GetProcessById((int)pid);
            var appName = proc.ProcessName;

            return (appName, windowTitle);
        }
        catch
        {
            return ("Unknown", "Unknown");
        }
    }

    public void Track()
    {
        var (app, window) = GetActiveWindow();

        lock (_lock)
        {
            if (app != _currentApp)
            {
                if (!string.IsNullOrEmpty(_currentApp))
                {
                    var duration = (int)(DateTime.UtcNow - _appStartTime).TotalSeconds;
                    if (duration > 2)
                    {
                        _records.Add(new AppUsageRecord
                        {
                            AppName = _currentApp,
                            WindowTitle = _currentWindow,
                            ProcessName = _currentApp,
                            StartTime = _appStartTime,
                            DurationSeconds = duration,
                            IsFocused = true
                        });
                    }
                }
                _currentApp = app;
                _currentWindow = window;
                _appStartTime = DateTime.UtcNow;
            }
            else
            {
                _currentWindow = window;
            }
        }
    }

    public List<AppUsageRecord> Flush()
    {
        lock (_lock)
        {
            if (!string.IsNullOrEmpty(_currentApp))
            {
                var duration = (int)(DateTime.UtcNow - _appStartTime).TotalSeconds;
                if (duration > 2)
                {
                    _records.Add(new AppUsageRecord
                    {
                        AppName = _currentApp,
                        WindowTitle = _currentWindow,
                        ProcessName = _currentApp,
                        StartTime = _appStartTime,
                        DurationSeconds = duration,
                        IsFocused = true
                    });
                }
                _appStartTime = DateTime.UtcNow;
            }

            var result = new List<AppUsageRecord>(_records);
            _records.Clear();
            return result;
        }
    }
}

public class AppUsageRecord
{
    public string AppName { get; set; } = "";
    public string WindowTitle { get; set; } = "";
    public string ProcessName { get; set; } = "";
    public DateTime StartTime { get; set; }
    public int DurationSeconds { get; set; }
    public bool IsFocused { get; set; }
}
