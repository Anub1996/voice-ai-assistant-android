using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using WorkSentinel.Server.Data;
using WorkSentinel.Server.Models;

namespace WorkSentinel.Server.Controllers;

[ApiController]
[Route("api/[controller]")]
[Authorize]
public class AlertsController : ControllerBase
{
    private readonly AppDbContext _db;

    public AlertsController(AppDbContext db) => _db = db;

    [HttpGet]
    public async Task<IActionResult> GetAlerts(
        [FromQuery] string? severity,
        [FromQuery] string? category,
        [FromQuery] bool? acknowledged,
        [FromQuery] int page = 1,
        [FromQuery] int pageSize = 50)
    {
        var q = _db.Alerts.Include(a => a.Employee).AsQueryable();
        if (!string.IsNullOrWhiteSpace(severity)) q = q.Where(a => a.Severity == severity);
        if (!string.IsNullOrWhiteSpace(category)) q = q.Where(a => a.Category == category);
        if (acknowledged.HasValue) q = q.Where(a => a.Acknowledged == acknowledged.Value);

        var total = await q.CountAsync();
        var items = await q
            .OrderByDescending(a => a.Timestamp)
            .Skip((page - 1) * pageSize)
            .Take(pageSize)
            .Select(a => new AlertDto(a.Id, a.Employee.FullName, a.Category, a.Severity, a.Title, a.Description, a.Acknowledged, a.Timestamp))
            .ToListAsync();

        return Ok(new { total, page, pageSize, items });
    }

    [HttpPost("{id}/acknowledge")]
    public async Task<IActionResult> Acknowledge(long id)
    {
        var alert = await _db.Alerts.FindAsync(id);
        if (alert == null) return NotFound();
        alert.Acknowledged = true;
        alert.AcknowledgedAt = DateTime.UtcNow;
        alert.AcknowledgedBy = User.Identity?.Name ?? "";
        await _db.SaveChangesAsync();
        return Ok();
    }

    [HttpPost("acknowledge-all")]
    public async Task<IActionResult> AcknowledgeAll()
    {
        await _db.Alerts
            .Where(a => !a.Acknowledged)
            .ExecuteUpdateAsync(s => s
                .SetProperty(a => a.Acknowledged, true)
                .SetProperty(a => a.AcknowledgedAt, DateTime.UtcNow)
                .SetProperty(a => a.AcknowledgedBy, User.Identity!.Name!));
        return Ok();
    }
}
