namespace ZingHR.Api.Models;

// ── Auth ──
public record LoginRequest(string Username, string Password);
public record PinLoginRequest(string Username, string Pin);
public record LoginResponse(string Token, string Username, string FullName, string Role, int? EmployeeId, DateTime ExpiresAt);
public record SetPinRequest(string Pin);

// ── Profile ──
public record ProfileDto
{
    public int Id { get; init; }
    public string EmpCode { get; init; } = "";
    public string FullName { get; init; } = "";
    public string Email { get; init; } = "";
    public string Phone { get; init; } = "";
    public string Gender { get; init; } = "";
    public string Department { get; init; } = "";
    public string Designation { get; init; } = "";
    public string ReportingManager { get; init; } = "";
    public string ProfilePhotoUrl { get; init; } = "";
    public DateOnly DateOfJoining { get; init; }
    public string EmploymentType { get; init; } = "";
    public string Status { get; init; } = "";
}

public record ProfileUpdateRequest(string? Phone, string? Address, string? City, string? State, string? PinCode, string? EmergencyContact);

// ── Attendance ──
public record PunchRequest(double Latitude, double Longitude, string? Address);
public record AttendanceDayDto(DateOnly Date, DateTime? PunchIn, DateTime? PunchOut, int WorkingMinutes, int LateByMinutes, string Status);
public record AttendanceSummaryDto(int PresentDays, int AbsentDays, int HalfDays, int LeaveDays, int Holidays, int WeekOffs, int LateDays, int TotalWorkingMinutes, double AvgWorkingHours);
public record RegularizeRequest(DateOnly Date, DateTime PunchIn, DateTime PunchOut, string Reason);

// ── Leave ──
public record LeaveApplyRequest(int LeaveTypeId, DateOnly FromDate, DateOnly ToDate, bool IsHalfDay, string? HalfDayType, string Reason, string? EmergencyContact);
public record LeaveBalanceDto(int LeaveTypeId, string LeaveTypeName, string Color, double Entitled, double Used, double Balance, bool IsPaid);
public record LeaveRequestDto
{
    public long Id { get; init; }
    public string LeaveType { get; init; } = "";
    public string LeaveTypeColor { get; init; } = "";
    public DateOnly FromDate { get; init; }
    public DateOnly ToDate { get; init; }
    public double TotalDays { get; init; }
    public bool IsHalfDay { get; init; }
    public string Reason { get; init; } = "";
    public string Status { get; init; } = "";
    public string? ApproverRemarks { get; init; }
    public DateTime AppliedAt { get; init; }
    public DateTime? ActionDate { get; init; }
}
public record LeaveActionRequest(string Action, string? Remarks); // approve, reject

// ── Payslip ──
public record PayslipListDto(long Id, string PayPeriod, int Month, int Year, decimal NetPay, string Status, DateTime GeneratedAt);
public record PayslipDetailDto
{
    public long Id { get; init; }
    public string PayPeriod { get; init; } = "";
    public string EmployeeName { get; init; } = "";
    public string EmpCode { get; init; } = "";
    public string Department { get; init; } = "";
    public string Designation { get; init; } = "";
    public int Month { get; init; }
    public int Year { get; init; }
    public decimal BasicSalary { get; init; }
    public decimal Hra { get; init; }
    public decimal ConveyanceAllowance { get; init; }
    public decimal MedicalAllowance { get; init; }
    public decimal SpecialAllowance { get; init; }
    public decimal OtherEarnings { get; init; }
    public decimal GrossEarnings { get; init; }
    public decimal Pf { get; init; }
    public decimal Esi { get; init; }
    public decimal ProfessionalTax { get; init; }
    public decimal Tds { get; init; }
    public decimal LoanDeduction { get; init; }
    public decimal OtherDeductions { get; init; }
    public decimal TotalDeductions { get; init; }
    public decimal NetPay { get; init; }
    public int PaidDays { get; init; }
    public int LopDays { get; init; }
    public string Status { get; init; } = "";
}

// ── Expense ──
public record ExpenseSubmitRequest(int CategoryId, string Title, string? Description, decimal Amount, DateOnly ExpenseDate, string? ReceiptUrl);
public record ExpenseDto
{
    public long Id { get; init; }
    public string Category { get; init; } = "";
    public string Title { get; init; } = "";
    public decimal Amount { get; init; }
    public DateOnly ExpenseDate { get; init; }
    public string Status { get; init; } = "";
    public decimal? ApprovedAmount { get; init; }
    public string? ApproverRemarks { get; init; }
    public DateTime SubmittedAt { get; init; }
}

// ── Directory ──
public record DirectoryEntryDto(int Id, string FullName, string EmpCode, string Department, string Designation, string Email, string Phone, string ProfilePhotoUrl);

// ── Help Desk ──
public record CreateTicketRequest(string Category, string Subject, string Description, string Priority);
public record TicketDto
{
    public long Id { get; init; }
    public string TicketNo { get; init; } = "";
    public string Category { get; init; } = "";
    public string Subject { get; init; } = "";
    public string Description { get; init; } = "";
    public string Priority { get; init; } = "";
    public string Status { get; init; } = "";
    public string? Resolution { get; init; }
    public DateTime CreatedAt { get; init; }
    public DateTime? ResolvedAt { get; init; }
    public List<TicketCommentDto> Comments { get; init; } = new();
}
public record TicketCommentDto(long Id, string UserName, string Comment, DateTime CreatedAt);
public record AddCommentRequest(string Comment);

// ── Notification ──
public record NotificationDto(long Id, string Title, string Body, string Type, bool IsRead, DateTime CreatedAt);

// ── Announcement ──
public record AnnouncementDto(int Id, string Title, string Content, string Category, string Priority, DateTime PublishedAt);

// ── Holiday ──
public record HolidayDto(int Id, string Name, DateOnly Date, string Type);

// ── Dashboard ──
public record EmployeeDashboardDto
{
    public string FullName { get; init; } = "";
    public string Department { get; init; } = "";
    public string Designation { get; init; } = "";
    public string ProfilePhotoUrl { get; init; } = "";
    public AttendanceDayDto? TodayAttendance { get; init; }
    public List<LeaveBalanceDto> LeaveBalance { get; init; } = new();
    public int PendingLeaves { get; init; }
    public int PendingExpenses { get; init; }
    public int UnreadNotifications { get; init; }
    public int OpenTickets { get; init; }
    public List<AnnouncementDto> RecentAnnouncements { get; init; } = new();
    public List<HolidayDto> UpcomingHolidays { get; init; } = new();
}

public record AdminDashboardDto
{
    public int TotalEmployees { get; init; }
    public int ActiveEmployees { get; init; }
    public int PresentToday { get; init; }
    public int AbsentToday { get; init; }
    public int OnLeaveToday { get; init; }
    public int PendingLeaveRequests { get; init; }
    public int PendingExpenses { get; init; }
    public int OpenTickets { get; init; }
    public List<DepartmentAttendanceDto> DepartmentAttendance { get; init; } = new();
    public List<AnnouncementDto> RecentAnnouncements { get; init; } = new();
}

public record DepartmentAttendanceDto(string Department, int Total, int Present, int Absent, int OnLeave);

// ── Manager Approvals ──
public record PendingApprovalsDto
{
    public List<LeaveRequestDto> PendingLeaves { get; init; } = new();
    public List<ExpenseDto> PendingExpenses { get; init; } = new();
    public List<RegularizeRequestDto> PendingRegularizations { get; init; } = new();
}

public record RegularizeRequestDto(long Id, string EmployeeName, DateOnly Date, DateTime PunchIn, DateTime PunchOut, string Reason, DateTime RequestedAt);
