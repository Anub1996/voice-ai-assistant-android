using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using WorkSentinel.Server.Data;
using WorkSentinel.Server.Models;

namespace WorkSentinel.Server.Controllers;

[ApiController]
[Route("api/[controller]")]
[Authorize]
public class DashboardController : ControllerBase
{
    private readonly AppDbContext _db;

    public DashboardController(AppDbContext db) => _db = db;

    [HttpGet("stats")]
    public async Task<IActionResult> GetStats()
    {
        var today = DateTime.UtcNow.Date;

        var employees = await _db.Employees.ToListAsync();
        var todayAlerts = await _db.Alerts.Where(a => a.Timestamp >= today).ToListAsync();
        var todayEmails = await _db.EmailLogs.CountAsync(e => e.Timestamp >= today);
        var todayWeb = await _db.WebVisits.CountAsync(w => w.VisitTime >= today);
        var scores = await _db.ProductivityScores
            .Where(p => p.Date == DateOnly.FromDateTime(today))
            .ToListAsync();

        var topApps = await _db.AppUsages
            .Where(a => a.StartTime >= today)
            .GroupBy(a => new { a.AppName, a.Category })
            .Select(g => new TopAppDto(g.Key.AppName, g.Key.Category, g.Sum(a => a.DurationSeconds) / 60, g.Select(a => a.EmployeeId).Distinct().Count()))
            .OrderByDescending(a => a.TotalMinutes)
            .Take(10)
            .ToListAsync();

        var topWeb = await _db.WebVisits
            .Where(w => w.VisitTime >= today)
            .GroupBy(w => new { w.Domain, w.Category })
            .Select(g => new TopWebDto(g.Key.Domain, g.Key.Category, g.Sum(w => w.DurationSeconds) / 60, g.Select(w => w.EmployeeId).Distinct().Count()))
            .OrderByDescending(w => w.TotalMinutes)
            .Take(10)
            .ToListAsync();

        var recentAlerts = await _db.Alerts
            .Include(a => a.Employee)
            .OrderByDescending(a => a.Timestamp)
            .Take(20)
            .Select(a => new AlertDto(a.Id, a.Employee.FullName, a.Category, a.Severity, a.Title, a.Description, a.Acknowledged, a.Timestamp))
            .ToListAsync();

        // 7-day trend
        var trend = new List<ProductivityTrend>();
        for (int i = 6; i >= 0; i--)
        {
            var d = DateOnly.FromDateTime(today.AddDays(-i));
            var dayScores = await _db.ProductivityScores.Where(p => p.Date == d).ToListAsync();
            trend.Add(new ProductivityTrend(
                d.ToString("MMM dd"),
                dayScores.Count > 0 ? Math.Round(dayScores.Average(s => s.Score), 1) : 0,
                dayScores.Sum(s => s.ActiveMinutes),
                dayScores.Sum(s => s.IdleMinutes)));
        }

        return Ok(new DashboardStats
        {
            TotalEmployees = employees.Count,
            OnlineEmployees = employees.Count(e => e.Status == "online"),
            IdleEmployees = employees.Count(e => e.Status == "idle"),
            AvgProductivityScore = scores.Count > 0 ? Math.Round(scores.Average(s => s.Score), 1) : 0,
            TotalAlertsToday = todayAlerts.Count,
            UnacknowledgedAlerts = todayAlerts.Count(a => !a.Acknowledged),
            TotalEmailsToday = todayEmails,
            TotalWebVisitsToday = todayWeb,
            TopApps = topApps,
            TopWebsites = topWeb,
            RecentAlerts = recentAlerts,
            ProductivityTrend = trend
        });
    }
}
