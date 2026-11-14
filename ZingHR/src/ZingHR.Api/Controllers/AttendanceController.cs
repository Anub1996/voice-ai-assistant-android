using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using ZingHR.Api.Data;
using ZingHR.Api.Models;
using ZingHR.Api.Services;

namespace ZingHR.Api.Controllers;

[ApiController]
[Route("api/[controller]")]
[Authorize]
public class AttendanceController : ControllerBase
{
    private readonly AppDbContext _db;
    private readonly NotificationService _notif;

    public AttendanceController(AppDbContext db, NotificationService notif)
    {
        _db = db;
        _notif = notif;
    }

    private int GetEmployeeId() =>
        int.Parse(User.FindFirstValue("employeeId") ?? "0");

    [HttpPost("punch-in")]
    public async Task<IActionResult> PunchIn([FromBody] PunchRequest req)
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var today = DateOnly.FromDateTime(DateTime.UtcNow);
        var existing = await _db.Attendances.FirstOrDefaultAsync(a => a.EmployeeId == empId && a.Date == today);
        if (existing?.PunchIn != null)
            return BadRequest(new { message = "Already punched in today" });

        var emp = await _db.Employees.Include(e => e.Shift).FirstOrDefaultAsync(e => e.Id == empId);
        var shift = emp?.Shift;
        var now = DateTime.UtcNow;
        int lateMinutes = 0;
        if (shift != null)
        {
            var shiftStart = today.ToDateTime(shift.StartTime, DateTimeKind.Utc);
            if (now > shiftStart.AddMinutes(shift.GraceLateMinutes))
                lateMinutes = (int)(now - shiftStart).TotalMinutes;
        }

        if (existing != null)
        {
            existing.PunchIn = now;
            existing.PunchInLatitude = req.Latitude;
            existing.PunchInLongitude = req.Longitude;
            existing.PunchInAddress = req.Address;
            existing.PunchInSource = "mobile";
            existing.LateByMinutes = lateMinutes;
            existing.Status = "present";
        }
        else
        {
            _db.Attendances.Add(new Attendance
            {
                EmployeeId = empId,
                Date = today,
                PunchIn = now,
                PunchInLatitude = req.Latitude,
                PunchInLongitude = req.Longitude,
                PunchInAddress = req.Address,
                PunchInSource = "mobile",
                LateByMinutes = lateMinutes,
                Status = "present"
            });
        }

        await _db.SaveChangesAsync();
        return Ok(new { message = "Punched in", time = now, lateByMinutes = lateMinutes });
    }

    [HttpPost("punch-out")]
    public async Task<IActionResult> PunchOut([FromBody] PunchRequest req)
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var today = DateOnly.FromDateTime(DateTime.UtcNow);
        var record = await _db.Attendances.FirstOrDefaultAsync(a => a.EmployeeId == empId && a.Date == today);
        if (record == null || record.PunchIn == null)
            return BadRequest(new { message = "Not punched in today" });
        if (record.PunchOut != null)
            return BadRequest(new { message = "Already punched out today" });

        var now = DateTime.UtcNow;
        record.PunchOut = now;
        record.PunchOutLatitude = req.Latitude;
        record.PunchOutLongitude = req.Longitude;
        record.PunchOutAddress = req.Address;
        record.PunchOutSource = "mobile";
        record.WorkingMinutes = (int)(now - record.PunchIn.Value).TotalMinutes;

        var emp = await _db.Employees.Include(e => e.Shift).FirstOrDefaultAsync(e => e.Id == empId);
        var shift = emp?.Shift;
        if (shift != null)
        {
            var shiftEnd = today.ToDateTime(shift.EndTime, DateTimeKind.Utc);
            if (now < shiftEnd.AddMinutes(-shift.GraceEarlyMinutes))
                record.EarlyGoingMinutes = (int)(shiftEnd - now).TotalMinutes;
            if (record.WorkingMinutes > shift.MinWorkingHours * 60)
                record.OvertimeMinutes = record.WorkingMinutes - shift.MinWorkingHours * 60;
        }

        await _db.SaveChangesAsync();
        return Ok(new { message = "Punched out", time = now, workingMinutes = record.WorkingMinutes });
    }

    [HttpGet("today")]
    public async Task<IActionResult> Today()
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var today = DateOnly.FromDateTime(DateTime.UtcNow);
        var record = await _db.Attendances.FirstOrDefaultAsync(a => a.EmployeeId == empId && a.Date == today);
        if (record == null)
            return Ok(new AttendanceDayDto(today, null, null, 0, 0, "not-punched"));

        return Ok(new AttendanceDayDto(record.Date, record.PunchIn, record.PunchOut, record.WorkingMinutes, record.LateByMinutes, record.Status));
    }

    [HttpGet("history")]
    public async Task<IActionResult> History([FromQuery] int month = 0, [FromQuery] int year = 0)
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        if (month == 0) month = DateTime.UtcNow.Month;
        if (year == 0) year = DateTime.UtcNow.Year;

        var start = new DateOnly(year, month, 1);
        var end = start.AddMonths(1);

        var records = await _db.Attendances
            .Where(a => a.EmployeeId == empId && a.Date >= start && a.Date < end)
            .OrderByDescending(a => a.Date)
            .Select(a => new AttendanceDayDto(a.Date, a.PunchIn, a.PunchOut, a.WorkingMinutes, a.LateByMinutes, a.Status))
            .ToListAsync();

        return Ok(records);
    }

    [HttpGet("summary")]
    public async Task<IActionResult> Summary([FromQuery] int month = 0, [FromQuery] int year = 0)
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        if (month == 0) month = DateTime.UtcNow.Month;
        if (year == 0) year = DateTime.UtcNow.Year;

        var start = new DateOnly(year, month, 1);
        var end = start.AddMonths(1);

        var records = await _db.Attendances
            .Where(a => a.EmployeeId == empId && a.Date >= start && a.Date < end)
            .ToListAsync();

        var holidays = await _db.Holidays
            .Where(h => h.Date >= start && h.Date < end && h.IsActive)
            .CountAsync();

        int weekOffs = 0;
        for (var d = start; d < end; d = d.AddDays(1))
            if (d.DayOfWeek is DayOfWeek.Saturday or DayOfWeek.Sunday) weekOffs++;

        return Ok(new AttendanceSummaryDto(
            PresentDays: records.Count(r => r.Status == "present"),
            AbsentDays: records.Count(r => r.Status == "absent"),
            HalfDays: records.Count(r => r.Status == "half-day"),
            LeaveDays: records.Count(r => r.Status == "on-leave"),
            Holidays: holidays,
            WeekOffs: weekOffs,
            LateDays: records.Count(r => r.LateByMinutes > 0),
            TotalWorkingMinutes: records.Sum(r => r.WorkingMinutes),
            AvgWorkingHours: records.Count > 0 ? Math.Round(records.Average(r => r.WorkingMinutes) / 60.0, 1) : 0
        ));
    }

    [HttpPost("regularize")]
    public async Task<IActionResult> Regularize([FromBody] RegularizeRequest req)
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var record = await _db.Attendances.FirstOrDefaultAsync(a => a.EmployeeId == empId && a.Date == req.Date);
        if (record == null)
        {
            record = new Attendance { EmployeeId = empId, Date = req.Date, Status = "present" };
            _db.Attendances.Add(record);
        }

        record.PunchIn = req.PunchIn;
        record.PunchOut = req.PunchOut;
        record.WorkingMinutes = (int)(req.PunchOut - req.PunchIn).TotalMinutes;
        record.Remarks = req.Reason;
        record.Status = "present";
        record.IsRegularized = true;
        record.RegularizedAt = DateTime.UtcNow;

        await _db.SaveChangesAsync();
        await _notif.SendToManagerAsync(empId, "Attendance Regularization", $"Regularization requested for {req.Date}", "attendance", record.Id);

        return Ok(new { message = "Regularization submitted" });
    }

    [Authorize(Roles = "admin,hr,manager")]
    [HttpGet("team")]
    public async Task<IActionResult> TeamAttendance([FromQuery] DateOnly? date)
    {
        var empId = GetEmployeeId();
        var targetDate = date ?? DateOnly.FromDateTime(DateTime.UtcNow);

        var subordinates = await _db.Employees
            .Where(e => e.ReportingManagerId == empId && e.IsActive)
            .ToListAsync();

        var empIds = subordinates.Select(e => e.Id).ToList();
        var records = await _db.Attendances
            .Where(a => empIds.Contains(a.EmployeeId) && a.Date == targetDate)
            .ToListAsync();

        var result = subordinates.Select(e =>
        {
            var att = records.FirstOrDefault(a => a.EmployeeId == e.Id);
            return new
            {
                e.Id, e.FullName, e.EmpCode,
                Status = att?.Status ?? "absent",
                PunchIn = att?.PunchIn,
                PunchOut = att?.PunchOut,
                WorkingMinutes = att?.WorkingMinutes ?? 0,
                LateByMinutes = att?.LateByMinutes ?? 0
            };
        });

        return Ok(result);
    }
}
