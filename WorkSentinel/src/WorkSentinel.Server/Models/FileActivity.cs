using System.ComponentModel.DataAnnotations;

namespace WorkSentinel.Server.Models;

public class FileActivity
{
    public long Id { get; set; }
    public int EmployeeId { get; set; }

    [Required, MaxLength(1000)]
    public string FilePath { get; set; } = "";

    [MaxLength(200)]
    public string FileName { get; set; } = "";

    [MaxLength(50)]
    public string Action { get; set; } = ""; // created, modified, deleted, copied, moved, usb_transfer, cloud_upload

    [MaxLength(200)]
    public string Destination { get; set; } = "";

    public long FileSizeBytes { get; set; }
    public DateTime Timestamp { get; set; }

    // Navigation
    public Employee Employee { get; set; } = null!;
}
