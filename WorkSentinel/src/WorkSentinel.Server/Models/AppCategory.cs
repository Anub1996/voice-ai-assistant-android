using System.ComponentModel.DataAnnotations;

namespace WorkSentinel.Server.Models;

public class AppCategory
{
    public int Id { get; set; }

    [Required, MaxLength(200)]
    public string AppNamePattern { get; set; } = "";

    [Required, MaxLength(50)]
    public string Category { get; set; } = "neutral"; // productive, unproductive, neutral

    [MaxLength(200)]
    public string DisplayName { get; set; } = "";
}

public class WebCategory
{
    public int Id { get; set; }

    [Required, MaxLength(200)]
    public string DomainPattern { get; set; } = "";

    [Required, MaxLength(50)]
    public string Category { get; set; } = "neutral";

    [MaxLength(200)]
    public string DisplayName { get; set; } = "";
}
