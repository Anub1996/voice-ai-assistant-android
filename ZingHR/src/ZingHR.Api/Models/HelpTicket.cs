using System.ComponentModel.DataAnnotations;

namespace ZingHR.Api.Models;

public class HelpTicket
{
    public long Id { get; set; }
    public int EmployeeId { get; set; }

    [MaxLength(20)]
    public string TicketNo { get; set; } = "";

    [Required, MaxLength(50)]
    public string Category { get; set; } = "general"; // it, hr, payroll, admin, general

    [Required, MaxLength(300)]
    public string Subject { get; set; } = "";

    [MaxLength(2000)]
    public string Description { get; set; } = "";

    [MaxLength(50)]
    public string Priority { get; set; } = "medium"; // low, medium, high, urgent

    [MaxLength(50)]
    public string Status { get; set; } = "open"; // open, in-progress, resolved, closed

    public int? AssignedTo { get; set; }

    [MaxLength(2000)]
    public string? Resolution { get; set; }

    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
    public DateTime? ResolvedAt { get; set; }
    public DateTime? ClosedAt { get; set; }

    public Employee Employee { get; set; } = null!;
    public ICollection<TicketComment> Comments { get; set; } = new List<TicketComment>();
}

public class TicketComment
{
    public long Id { get; set; }
    public long TicketId { get; set; }
    public int UserId { get; set; }

    [MaxLength(2000)]
    public string Comment { get; set; } = "";

    public bool IsInternal { get; set; }
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;

    public HelpTicket Ticket { get; set; } = null!;
}
