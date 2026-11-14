using System.ComponentModel.DataAnnotations;

namespace ZingHR.Api.Models;

public class LeaveType
{
    public int Id { get; set; }

    [Required, MaxLength(100)]
    public string Name { get; set; } = "";

    [MaxLength(20)]
    public string Code { get; set; } = "";

    public int DefaultDays { get; set; } = 12;
    public bool IsPaid { get; set; } = true;
    public bool IsCarryForward { get; set; }
    public int MaxCarryForward { get; set; }
    public bool IsEncashable { get; set; }
    public bool RequiresApproval { get; set; } = true;
    public bool AllowHalfDay { get; set; } = true;

    [MaxLength(20)]
    public string Color { get; set; } = "#4CAF50";

    public bool IsActive { get; set; } = true;

    public ICollection<LeaveBalance> Balances { get; set; } = new List<LeaveBalance>();
    public ICollection<LeaveRequest> Requests { get; set; } = new List<LeaveRequest>();
}

public class LeaveBalance
{
    public int Id { get; set; }
    public int EmployeeId { get; set; }
    public int LeaveTypeId { get; set; }
    public int Year { get; set; }
    public double Entitled { get; set; }
    public double Used { get; set; }
    public double CarriedForward { get; set; }
    public double Balance { get; set; }

    public Employee Employee { get; set; } = null!;
    public LeaveType LeaveType { get; set; } = null!;
}

public class LeaveRequest
{
    public long Id { get; set; }
    public int EmployeeId { get; set; }
    public int LeaveTypeId { get; set; }

    public DateOnly FromDate { get; set; }
    public DateOnly ToDate { get; set; }
    public double TotalDays { get; set; }

    public bool IsHalfDay { get; set; }

    [MaxLength(20)]
    public string? HalfDayType { get; set; } // first-half, second-half

    [MaxLength(1000)]
    public string Reason { get; set; } = "";

    [MaxLength(50)]
    public string Status { get; set; } = "pending"; // pending, approved, rejected, cancelled

    public int? ApprovedBy { get; set; }
    public DateTime? ActionDate { get; set; }

    [MaxLength(500)]
    public string? ApproverRemarks { get; set; }

    [MaxLength(500)]
    public string? EmergencyContact { get; set; }

    public DateTime AppliedAt { get; set; } = DateTime.UtcNow;

    public Employee Employee { get; set; } = null!;
    public LeaveType LeaveType { get; set; } = null!;
}
