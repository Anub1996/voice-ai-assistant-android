using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using WorkSentinel.Server.Data;
using WorkSentinel.Server.Models;

namespace WorkSentinel.Server.Controllers;

[ApiController]
[Route("api/[controller]")]
[Authorize]
public class ReportsController : ControllerBase
{
    private readonly AppDbContext _db;

    public ReportsController(AppDbContext db) => _db = db;

    [HttpGet("productivity")]
    public async Task<IActionResult> ProductivityReport([FromQuery] string period = "week")
    {
        var days = period switch { "month" => 30, "quarter" => 90, _ => 7 };
        var startDate = DateOnly.FromDateTime(DateTime.UtcNow.AddDays(-days));

        var employees = await _db.Employees.ToListAsync();
        var rows = new List<EmployeeProductivityRow>();

        foreach (var emp in employees)
        {
            var scores = await _db.ProductivityScores
                .Where(p => p.EmployeeId == emp.Id && p.Date >= startDate)
                .ToListAsync();

            var topApp = await _db.AppUsages
                .Where(a => a.EmployeeId == emp.Id && a.StartTime >= startDate.ToDateTime(TimeOnly.MinValue))
                .GroupBy(a => a.AppName)
                .OrderByDescending(g => g.Sum(a => a.DurationSeconds))
                .Select(g => g.Key)
                .FirstOrDefaultAsync();

            var topWeb = await _db.WebVisits
                .Where(w => w.EmployeeId == emp.Id && w.VisitTime >= startDate.ToDateTime(TimeOnly.MinValue))
                .GroupBy(w => w.Domain)
                .OrderByDescending(g => g.Sum(w => w.DurationSeconds))
                .Select(g => g.Key)
                .FirstOrDefaultAsync();

            var alerts = await _db.Alerts
                .CountAsync(a => a.EmployeeId == emp.Id && a.Timestamp >= startDate.ToDateTime(TimeOnly.MinValue));

            rows.Add(new EmployeeProductivityRow
            {
                EmployeeName = emp.FullName,
                Department = emp.Department,
                AvgScore = scores.Count > 0 ? Math.Round(scores.Average(s => s.Score), 1) : 0,
                TotalActiveHours = scores.Sum(s => s.ActiveMinutes) / 60,
                TotalIdleHours = scores.Sum(s => s.IdleMinutes) / 60,
                TopApp = topApp ?? "N/A",
                TopWebsite = topWeb ?? "N/A",
                Alerts = alerts
            });
        }

        return Ok(new ProductivityReport
        {
            Period = period,
            Rows = rows.OrderByDescending(r => r.AvgScore).ToList()
        });
    }

    [HttpGet("app-usage")]
    public async Task<IActionResult> AppUsageReport([FromQuery] int days = 7)
    {
        var since = DateTime.UtcNow.AddDays(-days);
        var data = await _db.AppUsages
            .Where(a => a.StartTime >= since)
            .GroupBy(a => new { a.AppName, a.Category })
            .Select(g => new
            {
                g.Key.AppName,
                g.Key.Category,
                TotalMinutes = g.Sum(a => a.DurationSeconds) / 60,
                EmployeeCount = g.Select(a => a.EmployeeId).Distinct().Count(),
                Sessions = g.Count()
            })
            .OrderByDescending(x => x.TotalMinutes)
            .Take(50)
            .ToListAsync();
        return Ok(data);
    }

    [HttpGet("web-usage")]
    public async Task<IActionResult> WebUsageReport([FromQuery] int days = 7)
    {
        var since = DateTime.UtcNow.AddDays(-days);
        var data = await _db.WebVisits
            .Where(w => w.VisitTime >= since)
            .GroupBy(w => new { w.Domain, w.Category })
            .Select(g => new
            {
                g.Key.Domain,
                g.Key.Category,
                TotalMinutes = g.Sum(w => w.DurationSeconds) / 60,
                EmployeeCount = g.Select(w => w.EmployeeId).Distinct().Count(),
                Visits = g.Count()
            })
            .OrderByDescending(x => x.TotalMinutes)
            .Take(50)
            .ToListAsync();
        return Ok(data);
    }

    [HttpGet("email-summary")]
    public async Task<IActionResult> EmailSummary([FromQuery] int days = 7)
    {
        var since = DateTime.UtcNow.AddDays(-days);
        var data = await _db.EmailLogs
            .Where(e => e.Timestamp >= since)
            .GroupBy(e => e.EmployeeId)
            .Select(g => new
            {
                EmployeeId = g.Key,
                Sent = g.Count(e => e.Direction == "outgoing"),
                Received = g.Count(e => e.Direction == "incoming"),
                Flagged = g.Count(e => e.Flagged),
                Alerts = g.Count(e => e.AlertTriggered)
            })
            .ToListAsync();

        var result = new List<object>();
        foreach (var d in data)
        {
            var emp = await _db.Employees.FindAsync(d.EmployeeId);
            result.Add(new { EmployeeName = emp?.FullName ?? "Unknown", d.Sent, d.Received, d.Flagged, d.Alerts });
        }
        return Ok(result);
    }
}
