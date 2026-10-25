using System.Runtime.InteropServices;

namespace WorkSentinel.Agent.Services;

public class KeystrokeService
{
    [DllImport("user32.dll")]
    private static extern short GetAsyncKeyState(int vKey);

    private int _keystrokeCount;
    private int _mouseClickCount;
    private readonly object _lock = new();

    public void Poll()
    {
        lock (_lock)
        {
            for (int key = 8; key <= 190; key++)
            {
                if ((GetAsyncKeyState(key) & 0x0001) != 0)
                    _keystrokeCount++;
            }

            if ((GetAsyncKeyState(0x01) & 0x0001) != 0) _mouseClickCount++;
            if ((GetAsyncKeyState(0x02) & 0x0001) != 0) _mouseClickCount++;
        }
    }

    public (int keystrokes, int clicks) Flush()
    {
        lock (_lock)
        {
            var result = (_keystrokeCount, _mouseClickCount);
            _keystrokeCount = 0;
            _mouseClickCount = 0;
            return result;
        }
    }
}
