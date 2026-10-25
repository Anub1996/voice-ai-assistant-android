using Microsoft.EntityFrameworkCore;
using WorkSentinel.Server.Data;
using WorkSentinel.Server.Models;

namespace WorkSentinel.Server.Services;

public class ProductivityService
{
    private readonly AppDbContext _db;

    public ProductivityService(AppDbContext db) => _db = db;

    public async Task<string> CategorizeAppAsync(string appName)
    {
        var categories = await _db.AppCategories.ToListAsync();
        var cat = categories.FirstOrDefault(c =>
            appName.Contains(c.AppNamePattern, StringComparison.OrdinalIgnoreCase));
        return cat?.Category ?? "neutral";
    }

    public async Task<string> CategorizeWebAsync(string domain)
    {
        var categories = await _db.WebCategories.ToListAsync();
        var cat = categories.FirstOrDefault(c =>
            domain.Contains(c.DomainPattern, StringComparison.OrdinalIgnoreCase));
        return cat?.Category ?? "neutral";
    }

    public async Task RecalculateDailyScoreAsync(int employeeId, DateOnly date)
    {
        var dayStart = date.ToDateTime(TimeOnly.MinValue, DateTimeKind.Utc);
        var dayEnd = date.AddDays(1).ToDateTime(TimeOnly.MinValue, DateTimeKind.Utc);

        var activities = await _db.ActivityLogs
            .Where(a => a.EmployeeId == employeeId && a.StartTime >= dayStart && a.StartTime < dayEnd)
            .ToListAsync();

        var apps = await _db.AppUsages
            .Where(a => a.EmployeeId == employeeId && a.StartTime >= dayStart && a.StartTime < dayEnd)
            .ToListAsync();

        var web = await _db.WebVisits
            .Where(w => w.EmployeeId == employeeId && w.VisitTime >= dayStart && w.VisitTime < dayEnd)
            .ToListAsync();

        var emails = await _db.EmailLogs
            .Where(e => e.EmployeeId == employeeId && e.Timestamp >= dayStart && e.Timestamp < dayEnd)
            .ToListAsync();

        var alerts = await _db.Alerts
            .Where(a => a.EmployeeId == employeeId && a.Timestamp >= dayStart && a.Timestamp < dayEnd)
            .CountAsync();

        int activeMin = activities.Where(a => a.ActivityType == "active").Sum(a => a.DurationSeconds) / 60;
        int idleMin = activities.Where(a => a.ActivityType == "idle").Sum(a => a.DurationSeconds) / 60;
        int productiveMin = apps.Where(a => a.Category == "productive").Sum(a => a.DurationSeconds) / 60;
        int unproductiveMin = apps.Where(a => a.Category == "unproductive").Sum(a => a.DurationSeconds) / 60;
        int neutralMin = apps.Where(a => a.Category == "neutral").Sum(a => a.DurationSeconds) / 60;

        int totalActive = activeMin + idleMin;
        double score = totalActive > 0
            ? Math.Clamp((double)productiveMin / totalActive * 100, 0, 100)
            : 0;

        var existing = await _db.ProductivityScores
            .FirstOrDefaultAsync(p => p.EmployeeId == employeeId && p.Date == date);

        if (existing != null)
        {
            existing.Score = score;
            existing.ActiveMinutes = activeMin;
            existing.IdleMinutes = idleMin;
            existing.ProductiveMinutes = productiveMin;
            existing.UnproductiveMinutes = unproductiveMin;
            existing.NeutralMinutes = neutralMin;
            existing.TotalAppsUsed = apps.Select(a => a.AppName).Distinct().Count();
            existing.TotalWebsitesVisited = web.Select(w => w.Domain).Distinct().Count();
            existing.TotalEmailsSent = emails.Count(e => e.Direction == "outgoing");
            existing.TotalEmailsReceived = emails.Count(e => e.Direction == "incoming");
            existing.AlertsTriggered = alerts;
        }
        else
        {
            _db.ProductivityScores.Add(new ProductivityScore
            {
                EmployeeId = employeeId,
                Date = date,
                Score = score,
                ActiveMinutes = activeMin,
                IdleMinutes = idleMin,
                ProductiveMinutes = productiveMin,
                UnproductiveMinutes = unproductiveMin,
                NeutralMinutes = neutralMin,
                TotalAppsUsed = apps.Select(a => a.AppName).Distinct().Count(),
                TotalWebsitesVisited = web.Select(w => w.Domain).Distinct().Count(),
                TotalEmailsSent = emails.Count(e => e.Direction == "outgoing"),
                TotalEmailsReceived = emails.Count(e => e.Direction == "incoming"),
                AlertsTriggered = alerts
            });
        }

        await _db.SaveChangesAsync();
    }
}
