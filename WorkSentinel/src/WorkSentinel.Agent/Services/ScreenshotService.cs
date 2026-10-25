using System;
using System.Drawing;
using System.Drawing.Imaging;
using System.IO;
using System.Linq;
using System.Runtime.InteropServices;

namespace WorkSentinel.Agent.Services;

public class ScreenshotService
{
    [DllImport("user32.dll")]
    private static extern int GetSystemMetrics(int nIndex);

    private const int SM_CXSCREEN = 0;
    private const int SM_CYSCREEN = 1;

    private readonly int _quality;

    public ScreenshotService(int quality = 50)
    {
        _quality = Math.Clamp(quality, 10, 100);
    }

    public byte[]? CaptureScreen()
    {
        try
        {
            int w = GetSystemMetrics(SM_CXSCREEN);
            int h = GetSystemMetrics(SM_CYSCREEN);
            if (w <= 0 || h <= 0) return null;

            using var bmp = new Bitmap(w, h);
            using var g = Graphics.FromImage(bmp);
            g.CopyFromScreen(0, 0, 0, 0, new Size(w, h));

            using var ms = new MemoryStream();
            var encoder = ImageCodecInfo.GetImageEncoders().First(c => c.FormatID == ImageFormat.Jpeg.Guid);
            var encoderParams = new EncoderParameters(1);
            encoderParams.Param[0] = new EncoderParameter(Encoder.Quality, (long)_quality);
            bmp.Save(ms, encoder, encoderParams);
            return ms.ToArray();
        }
        catch
        {
            return null;
        }
    }
}
