using System.ComponentModel.DataAnnotations;

namespace ZingHR.Api.Models;

public class Employee
{
    public int Id { get; set; }

    [Required, MaxLength(20)]
    public string EmpCode { get; set; } = "";

    [Required, MaxLength(200)]
    public string FullName { get; set; } = "";

    [MaxLength(100)]
    public string FirstName { get; set; } = "";

    [MaxLength(100)]
    public string LastName { get; set; } = "";

    [Required, MaxLength(200)]
    public string Email { get; set; } = "";

    [MaxLength(20)]
    public string Phone { get; set; } = "";

    [MaxLength(10)]
    public string Gender { get; set; } = "";

    public DateOnly? DateOfBirth { get; set; }
    public DateOnly DateOfJoining { get; set; }
    public DateOnly? DateOfExit { get; set; }

    public int DepartmentId { get; set; }
    public int DesignationId { get; set; }
    public int? ReportingManagerId { get; set; }
    public int? ShiftId { get; set; }

    [MaxLength(500)]
    public string Address { get; set; } = "";

    [MaxLength(100)]
    public string City { get; set; } = "";

    [MaxLength(100)]
    public string State { get; set; } = "";

    [MaxLength(20)]
    public string PinCode { get; set; } = "";

    [MaxLength(20)]
    public string PanNumber { get; set; } = "";

    [MaxLength(20)]
    public string AadharNumber { get; set; } = "";

    [MaxLength(50)]
    public string BankAccountNo { get; set; } = "";

    [MaxLength(20)]
    public string IfscCode { get; set; } = "";

    [MaxLength(200)]
    public string BankName { get; set; } = "";

    [MaxLength(20)]
    public string UanNumber { get; set; } = "";

    [MaxLength(20)]
    public string EsiNumber { get; set; } = "";

    [MaxLength(500)]
    public string ProfilePhotoUrl { get; set; } = "";

    [MaxLength(50)]
    public string EmploymentType { get; set; } = "full-time";

    [MaxLength(50)]
    public string Status { get; set; } = "active";

    public bool IsActive { get; set; } = true;
    public DateTime CreatedAt { get; set; } = DateTime.UtcNow;
    public DateTime? UpdatedAt { get; set; }

    // Navigation
    public Department Department { get; set; } = null!;
    public Designation Designation { get; set; } = null!;
    public Employee? ReportingManager { get; set; }
    public Shift? Shift { get; set; }
    public ICollection<Attendance> Attendances { get; set; } = new List<Attendance>();
    public ICollection<LeaveRequest> LeaveRequests { get; set; } = new List<LeaveRequest>();
    public ICollection<LeaveBalance> LeaveBalances { get; set; } = new List<LeaveBalance>();
    public ICollection<Payslip> Payslips { get; set; } = new List<Payslip>();
    public ICollection<Expense> Expenses { get; set; } = new List<Expense>();
    public ICollection<HelpTicket> HelpTickets { get; set; } = new List<HelpTicket>();
    public ICollection<Notification> Notifications { get; set; } = new List<Notification>();
}
