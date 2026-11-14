using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using ZingHR.Api.Data;
using ZingHR.Api.Models;

namespace ZingHR.Api.Controllers;

[ApiController]
[Route("api/[controller]")]
[Authorize]
public class DashboardController : ControllerBase
{
    private readonly AppDbContext _db;

    public DashboardController(AppDbContext db) => _db = db;

    private int GetEmployeeId() =>
        int.Parse(User.FindFirstValue("employeeId") ?? "0");

    [HttpGet("employee")]
    public async Task<IActionResult> EmployeeDashboard()
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var emp = await _db.Employees.Include(e => e.Department).Include(e => e.Designation)
            .FirstOrDefaultAsync(e => e.Id == empId);
        if (emp == null) return NotFound();

        var today = DateOnly.FromDateTime(DateTime.UtcNow);
        var todayAtt = await _db.Attendances.FirstOrDefaultAsync(a => a.EmployeeId == empId && a.Date == today);

        var leaveBalances = await _db.LeaveBalances
            .Include(b => b.LeaveType)
            .Where(b => b.EmployeeId == empId && b.Year == today.Year)
            .Select(b => new LeaveBalanceDto(b.LeaveTypeId, b.LeaveType.Name, b.LeaveType.Color, b.Entitled, b.Used, b.Balance, b.LeaveType.IsPaid))
            .ToListAsync();

        var pendingLeaves = await _db.LeaveRequests.CountAsync(l => l.EmployeeId == empId && l.Status == "pending");
        var pendingExpenses = await _db.Expenses.CountAsync(e => e.EmployeeId == empId && e.Status == "pending");
        var unreadNotifs = await _db.Notifications.CountAsync(n => n.EmployeeId == empId && !n.IsRead);
        var openTickets = await _db.HelpTickets.CountAsync(t => t.EmployeeId == empId && (t.Status == "open" || t.Status == "in-progress"));

        var announcements = await _db.Announcements
            .Where(a => a.IsPublished && (a.ExpiresAt == null || a.ExpiresAt > DateTime.UtcNow))
            .OrderByDescending(a => a.PublishedAt).Take(5)
            .Select(a => new AnnouncementDto(a.Id, a.Title, a.Content, a.Category, a.Priority, a.PublishedAt))
            .ToListAsync();

        var upcomingHolidays = await _db.Holidays
            .Where(h => h.Date >= today && h.IsActive)
            .OrderBy(h => h.Date).Take(5)
            .Select(h => new HolidayDto(h.Id, h.Name, h.Date, h.Type))
            .ToListAsync();

        return Ok(new EmployeeDashboardDto
        {
            FullName = emp.FullName,
            Department = emp.Department.Name,
            Designation = emp.Designation.Title,
            ProfilePhotoUrl = emp.ProfilePhotoUrl,
            TodayAttendance = todayAtt != null
                ? new AttendanceDayDto(todayAtt.Date, todayAtt.PunchIn, todayAtt.PunchOut, todayAtt.WorkingMinutes, todayAtt.LateByMinutes, todayAtt.Status)
                : null,
            LeaveBalance = leaveBalances,
            PendingLeaves = pendingLeaves,
            PendingExpenses = pendingExpenses,
            UnreadNotifications = unreadNotifs,
            OpenTickets = openTickets,
            RecentAnnouncements = announcements,
            UpcomingHolidays = upcomingHolidays
        });
    }

    [Authorize(Roles = "admin,hr")]
    [HttpGet("admin")]
    public async Task<IActionResult> AdminDashboard()
    {
        var today = DateOnly.FromDateTime(DateTime.UtcNow);

        var totalEmployees = await _db.Employees.CountAsync(e => e.IsActive);
        var activeEmployees = await _db.Employees.CountAsync(e => e.IsActive && e.Status == "active");

        var todayAttendances = await _db.Attendances.Where(a => a.Date == today).ToListAsync();
        var presentToday = todayAttendances.Count(a => a.Status == "present");
        var absentToday = totalEmployees - presentToday;
        var onLeave = todayAttendances.Count(a => a.Status == "on-leave");

        var pendingLeaves = await _db.LeaveRequests.CountAsync(l => l.Status == "pending");
        var pendingExpenses = await _db.Expenses.CountAsync(e => e.Status == "pending");
        var openTickets = await _db.HelpTickets.CountAsync(t => t.Status == "open" || t.Status == "in-progress");

        var departments = await _db.Departments.Where(d => d.IsActive)
            .Select(d => new
            {
                d.Name,
                Total = d.Employees.Count(e => e.IsActive),
                EmpIds = d.Employees.Where(e => e.IsActive).Select(e => e.Id).ToList()
            })
            .ToListAsync();

        var deptAttendance = departments.Select(d => new DepartmentAttendanceDto(
            d.Name,
            d.Total,
            todayAttendances.Count(a => d.EmpIds.Contains(a.EmployeeId) && a.Status == "present"),
            d.Total - todayAttendances.Count(a => d.EmpIds.Contains(a.EmployeeId)),
            todayAttendances.Count(a => d.EmpIds.Contains(a.EmployeeId) && a.Status == "on-leave")
        )).ToList();

        var announcements = await _db.Announcements
            .Where(a => a.IsPublished)
            .OrderByDescending(a => a.PublishedAt).Take(5)
            .Select(a => new AnnouncementDto(a.Id, a.Title, a.Content, a.Category, a.Priority, a.PublishedAt))
            .ToListAsync();

        return Ok(new AdminDashboardDto
        {
            TotalEmployees = totalEmployees,
            ActiveEmployees = activeEmployees,
            PresentToday = presentToday,
            AbsentToday = absentToday,
            OnLeaveToday = onLeave,
            PendingLeaveRequests = pendingLeaves,
            PendingExpenses = pendingExpenses,
            OpenTickets = openTickets,
            DepartmentAttendance = deptAttendance,
            RecentAnnouncements = announcements
        });
    }
}
