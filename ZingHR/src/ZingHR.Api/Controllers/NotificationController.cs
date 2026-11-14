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
public class NotificationController : ControllerBase
{
    private readonly AppDbContext _db;

    public NotificationController(AppDbContext db) => _db = db;

    private int GetEmployeeId() =>
        int.Parse(User.FindFirstValue("employeeId") ?? "0");

    [HttpGet]
    public async Task<IActionResult> GetAll([FromQuery] bool? unreadOnly, [FromQuery] int page = 1, [FromQuery] int pageSize = 20)
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var query = _db.Notifications.Where(n => n.EmployeeId == empId);
        if (unreadOnly == true)
            query = query.Where(n => !n.IsRead);

        var total = await query.CountAsync();
        var notifications = await query.OrderByDescending(n => n.CreatedAt)
            .Skip((page - 1) * pageSize).Take(pageSize)
            .Select(n => new NotificationDto(n.Id, n.Title, n.Body, n.Type, n.IsRead, n.CreatedAt))
            .ToListAsync();

        return Ok(new { total, page, pageSize, data = notifications });
    }

    [HttpGet("unread-count")]
    public async Task<IActionResult> UnreadCount()
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();
        var count = await _db.Notifications.CountAsync(n => n.EmployeeId == empId && !n.IsRead);
        return Ok(new { count });
    }

    [HttpPut("{id}/read")]
    public async Task<IActionResult> MarkRead(long id)
    {
        var empId = GetEmployeeId();
        var notif = await _db.Notifications.FirstOrDefaultAsync(n => n.Id == id && n.EmployeeId == empId);
        if (notif == null) return NotFound();

        notif.IsRead = true;
        notif.ReadAt = DateTime.UtcNow;
        await _db.SaveChangesAsync();
        return Ok();
    }

    [HttpPut("read-all")]
    public async Task<IActionResult> MarkAllRead()
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var unread = await _db.Notifications
            .Where(n => n.EmployeeId == empId && !n.IsRead)
            .ToListAsync();

        foreach (var n in unread)
        {
            n.IsRead = true;
            n.ReadAt = DateTime.UtcNow;
        }

        await _db.SaveChangesAsync();
        return Ok(new { markedRead = unread.Count });
    }
}

[ApiController]
[Route("api/[controller]")]
[Authorize]
public class AnnouncementController : ControllerBase
{
    private readonly AppDbContext _db;

    public AnnouncementController(AppDbContext db) => _db = db;

    [HttpGet]
    public async Task<IActionResult> GetAll()
    {
        var announcements = await _db.Announcements
            .Where(a => a.IsPublished && (a.ExpiresAt == null || a.ExpiresAt > DateTime.UtcNow))
            .OrderByDescending(a => a.PublishedAt)
            .Select(a => new AnnouncementDto(a.Id, a.Title, a.Content, a.Category, a.Priority, a.PublishedAt))
            .ToListAsync();
        return Ok(announcements);
    }

    [Authorize(Roles = "admin,hr")]
    [HttpPost]
    public async Task<IActionResult> Create([FromBody] CreateAnnouncementRequest req)
    {
        var userId = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);
        var announcement = new Announcement
        {
            Title = req.Title,
            Content = req.Content,
            Category = req.Category,
            Priority = req.Priority,
            IsPublished = true,
            CreatedBy = userId,
            ExpiresAt = req.ExpiresAt
        };

        _db.Announcements.Add(announcement);
        await _db.SaveChangesAsync();
        return Ok(new { message = "Announcement published", id = announcement.Id });
    }
}

public record CreateAnnouncementRequest(string Title, string Content, string Category, string Priority, DateTime? ExpiresAt);

[ApiController]
[Route("api/[controller]")]
[Authorize]
public class HolidayController : ControllerBase
{
    private readonly AppDbContext _db;

    public HolidayController(AppDbContext db) => _db = db;

    [HttpGet]
    public async Task<IActionResult> GetAll([FromQuery] int? year)
    {
        var y = year ?? DateTime.UtcNow.Year;
        var holidays = await _db.Holidays
            .Where(h => h.Year == y && h.IsActive)
            .OrderBy(h => h.Date)
            .Select(h => new HolidayDto(h.Id, h.Name, h.Date, h.Type))
            .ToListAsync();
        return Ok(holidays);
    }

    [HttpGet("upcoming")]
    public async Task<IActionResult> Upcoming()
    {
        var today = DateOnly.FromDateTime(DateTime.UtcNow);
        var holidays = await _db.Holidays
            .Where(h => h.Date >= today && h.IsActive)
            .OrderBy(h => h.Date)
            .Take(5)
            .Select(h => new HolidayDto(h.Id, h.Name, h.Date, h.Type))
            .ToListAsync();
        return Ok(holidays);
    }
}
