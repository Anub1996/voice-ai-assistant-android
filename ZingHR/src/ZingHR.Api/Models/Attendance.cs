using System.ComponentModel.DataAnnotations;

namespace ZingHR.Api.Models;

public class Attendance
{
    public long Id { get; set; }
    public int EmployeeId { get; set; }
    public DateOnly Date { get; set; }

    public DateTime? PunchIn { get; set; }
    public DateTime? PunchOut { get; set; }

    public double? PunchInLatitude { get; set; }
    public double? PunchInLongitude { get; set; }

    [MaxLength(500)]
    public string? PunchInAddress { get; set; }

    public double? PunchOutLatitude { get; set; }
    public double? PunchOutLongitude { get; set; }

    [MaxLength(500)]
    public string? PunchOutAddress { get; set; }

    [MaxLength(50)]
    public string PunchInSource { get; set; } = "mobile"; // mobile, web, biometric, manager

    [MaxLength(50)]
    public string? PunchOutSource { get; set; }

    public int WorkingMinutes { get; set; }
    public int OvertimeMinutes { get; set; }
    public int LateByMinutes { get; set; }
    public int EarlyGoingMinutes { get; set; }

    [MaxLength(50)]
    public string Status { get; set; } = "present"; // present, absent, half-day, weekend, holiday, on-leave, comp-off

    [MaxLength(500)]
    public string? Remarks { get; set; }

    public bool IsRegularized { get; set; }
    public DateTime? RegularizedAt { get; set; }
    public int? RegularizedBy { get; set; }

    public Employee Employee { get; set; } = null!;
}

public class Shift
{
    public int Id { get; set; }

    [Required, MaxLength(100)]
    public string Name { get; set; } = "";

    [MaxLength(20)]
    public string Code { get; set; } = "";

    public TimeOnly StartTime { get; set; }
    public TimeOnly EndTime { get; set; }
    public int GraceLateMinutes { get; set; } = 15;
    public int GraceEarlyMinutes { get; set; } = 15;
    public int MinWorkingHours { get; set; } = 8;
    public bool IsNightShift { get; set; }
    public bool IsActive { get; set; } = true;

    public ICollection<Employee> Employees { get; set; } = new List<Employee>();
}
