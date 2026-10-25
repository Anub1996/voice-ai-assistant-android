using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using WorkSentinel.Server.Data;
using WorkSentinel.Server.Models;

namespace WorkSentinel.Server.Controllers;

[ApiController]
[Route("api/[controller]")]
[Authorize]
public class EmployeesController : ControllerBase
{
    private readonly AppDbContext _db;

    public EmployeesController(AppDbContext db) => _db = db;

    [HttpGet]
    public async Task<IActionResult> GetAll([FromQuery] string? search, [FromQuery] string? department, [FromQuery] string? status)
    {
        var q = _db.Employees.AsQueryable();
        if (!string.IsNullOrWhiteSpace(search))
            q = q.Where(e => e.FullName.Contains(search) || e.Email.Contains(search) || e.EmployeeId.Contains(search));
        if (!string.IsNullOrWhiteSpace(department))
            q = q.Where(e => e.Department == department);
        if (!string.IsNullOrWhiteSpace(status))
            q = q.Where(e => e.Status == status);

        var employees = await q.OrderBy(e => e.FullName).ToListAsync();
        var today = DateOnly.FromDateTime(DateTime.UtcNow);

        var result = new List<object>();
        foreach (var e in employees)
        {
            var score = await _db.ProductivityScores
                .FirstOrDefaultAsync(p => p.EmployeeId == e.Id && p.Date == today);
            var alertCount = await _db.Alerts
                .CountAsync(a => a.EmployeeId == e.Id && a.Timestamp >= DateTime.UtcNow.Date && !a.Acknowledged);

            result.Add(new
            {
                e.Id, e.EmployeeId, e.FullName, e.Email, e.Department, e.Designation,
                e.ComputerName, e.Status, e.LastSeenAt, e.MonitoringEnabled,
                ProductivityScore = score?.Score ?? 0,
                ActiveMinutes = score?.ActiveMinutes ?? 0,
                AlertsToday = alertCount
            });
        }
        return Ok(result);
    }

    [HttpGet("{id}")]
    public async Task<IActionResult> GetDetail(int id, [FromQuery] string? date)
    {
        var emp = await _db.Employees.FindAsync(id);
        if (emp == null) return NotFound();

        var targetDate = date != null ? DateOnly.Parse(date) : DateOnly.FromDateTime(DateTime.UtcNow);
        var dayStart = targetDate.ToDateTime(TimeOnly.MinValue, DateTimeKind.Utc);
        var dayEnd = targetDate.AddDays(1).ToDateTime(TimeOnly.MinValue, DateTimeKind.Utc);

        var score = await _db.ProductivityScores
            .FirstOrDefaultAsync(p => p.EmployeeId == id && p.Date == targetDate);
        var alertCount = await _db.Alerts
            .CountAsync(a => a.EmployeeId == id && a.Timestamp >= dayStart && a.Timestamp < dayEnd);

        // Build timeline
        var timeline = new List<TimelineEntry>();

        var activities = await _db.ActivityLogs
            .Where(a => a.EmployeeId == id && a.StartTime >= dayStart && a.StartTime < dayEnd)
            .OrderByDescending(a => a.StartTime).Take(50).ToListAsync();
        foreach (var a in activities)
            timeline.Add(new TimelineEntry(a.StartTime, "activity", $"{a.ActivityType}: {a.ActiveApp}", a.ActiveWindow));

        var apps = await _db.AppUsages
            .Where(a => a.EmployeeId == id && a.StartTime >= dayStart && a.StartTime < dayEnd)
            .OrderByDescending(a => a.StartTime).Take(50).ToListAsync();
        foreach (var a in apps)
            timeline.Add(new TimelineEntry(a.StartTime, "app", a.AppName, $"{a.DurationSeconds / 60}m - {a.WindowTitle}"));

        var web = await _db.WebVisits
            .Where(w => w.EmployeeId == id && w.VisitTime >= dayStart && w.VisitTime < dayEnd)
            .OrderByDescending(w => w.VisitTime).Take(50).ToListAsync();
        foreach (var w in web)
            timeline.Add(new TimelineEntry(w.VisitTime, "web", w.Domain, w.Title));

        var emails = await _db.EmailLogs
            .Where(e => e.EmployeeId == id && e.Timestamp >= dayStart && e.Timestamp < dayEnd)
            .OrderByDescending(e => e.Timestamp).Take(50).ToListAsync();
        foreach (var e in emails)
            timeline.Add(new TimelineEntry(e.Timestamp, "email", $"{e.Direction}: {e.Subject}", $"{e.Sender} -> {e.Recipients}"));

        var files = await _db.FileActivities
            .Where(f => f.EmployeeId == id && f.Timestamp >= dayStart && f.Timestamp < dayEnd)
            .OrderByDescending(f => f.Timestamp).Take(50).ToListAsync();
        foreach (var f in files)
            timeline.Add(new TimelineEntry(f.Timestamp, "file", $"{f.Action}: {f.FileName}", f.FilePath));

        return Ok(new EmployeeDetailDto
        {
            Id = emp.Id,
            FullName = emp.FullName,
            Email = emp.Email,
            Department = emp.Department,
            ComputerName = emp.ComputerName,
            Status = emp.Status,
            LastSeenAt = emp.LastSeenAt,
            TodayProductivity = score?.Score ?? 0,
            ActiveMinutesToday = score?.ActiveMinutes ?? 0,
            IdleMinutesToday = score?.IdleMinutes ?? 0,
            AlertsToday = alertCount,
            Timeline = timeline.OrderByDescending(t => t.Time).Take(100).ToList()
        });
    }

    [HttpPut("{id}/monitoring")]
    public async Task<IActionResult> ToggleMonitoring(int id, [FromBody] bool enabled)
    {
        var emp = await _db.Employees.FindAsync(id);
        if (emp == null) return NotFound();
        emp.MonitoringEnabled = enabled;
        await _db.SaveChangesAsync();
        return Ok();
    }
}
