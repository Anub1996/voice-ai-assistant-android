using System.ComponentModel.DataAnnotations;

namespace WorkSentinel.Server.Models;

public class ScreenCapture
{
    public long Id { get; set; }
    public int EmployeeId { get; set; }

    [MaxLength(500)]
    public string FilePath { get; set; } = "";

    [MaxLength(200)]
    public string ActiveWindow { get; set; } = "";

    [MaxLength(200)]
    public string ActiveApp { get; set; } = "";

    public long FileSizeBytes { get; set; }
    public DateTime CapturedAt { get; set; }

    // Navigation
    public Employee Employee { get; set; } = null!;
}
