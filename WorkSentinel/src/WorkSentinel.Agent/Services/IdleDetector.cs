using System.Runtime.InteropServices;

namespace WorkSentinel.Agent.Services;

public class IdleDetector
{
    [DllImport("user32.dll")]
    private static extern bool GetLastInputInfo(ref LASTINPUTINFO plii);

    [StructLayout(LayoutKind.Sequential)]
    private struct LASTINPUTINFO
    {
        public uint cbSize;
        public uint dwTime;
    }

    private readonly int _idleThresholdSeconds;
    private DateTime _idleStart;
    private bool _isIdle;
    private readonly List<ActivityRecord> _records = new();
    private readonly object _lock = new();

    public IdleDetector(int idleThresholdSeconds)
    {
        _idleThresholdSeconds = idleThresholdSeconds;
    }

    public int GetIdleSeconds()
    {
        var info = new LASTINPUTINFO { cbSize = (uint)Marshal.SizeOf<LASTINPUTINFO>() };
        if (!GetLastInputInfo(ref info)) return 0;
        return (int)((Environment.TickCount - info.dwTime) / 1000);
    }

    public bool IsIdle => _isIdle;

    public string GetStatus()
    {
        var idle = GetIdleSeconds();
        var wasIdle = _isIdle;
        _isIdle = idle >= _idleThresholdSeconds;

        lock (_lock)
        {
            if (_isIdle && !wasIdle)
            {
                _idleStart = DateTime.UtcNow.AddSeconds(-idle);
            }
            else if (!_isIdle && wasIdle)
            {
                var duration = (int)(DateTime.UtcNow - _idleStart).TotalSeconds;
                _records.Add(new ActivityRecord
                {
                    ActivityType = "idle",
                    StartTime = _idleStart,
                    EndTime = DateTime.UtcNow,
                    DurationSeconds = duration
                });
            }
        }

        return _isIdle ? "idle" : "online";
    }

    public List<ActivityRecord> Flush()
    {
        lock (_lock)
        {
            var result = new List<ActivityRecord>(_records);
            _records.Clear();
            return result;
        }
    }
}

public class ActivityRecord
{
    public string ActivityType { get; set; } = "";
    public DateTime StartTime { get; set; }
    public DateTime EndTime { get; set; }
    public int DurationSeconds { get; set; }
    public string ActiveWindow { get; set; } = "";
    public string ActiveApp { get; set; } = "";
    public int KeystrokeCount { get; set; }
    public int MouseClickCount { get; set; }
}
