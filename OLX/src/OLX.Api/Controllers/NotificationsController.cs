using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using OLX.Api.Data;
using OLX.Api.Models;

namespace OLX.Api.Controllers;

[ApiController, Route("api/[controller]"), Authorize]
public class NotificationsController : ControllerBase
{
    private readonly AppDbContext _db;

    public NotificationsController(AppDbContext db) => _db = db;

    private int Uid => int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);

    [HttpGet]
    public async Task<IActionResult> GetAll()
    {
        var items = await _db.Notifications.Where(n => n.UserId == Uid)
            .OrderByDescending(n => n.CreatedAt).Take(50).ToListAsync();

        return Ok(items.Select(n => new NotificationDto(n.Id, n.Type, n.Title, n.Body, n.ListingId, n.ConversationId, n.IsRead, n.CreatedAt)));
    }

    [HttpGet("unread-count")]
    public async Task<IActionResult> GetUnreadCount()
    {
        var count = await _db.Notifications.CountAsync(n => n.UserId == Uid && !n.IsRead);
        return Ok(new { count });
    }

    [HttpPut("{id}/read")]
    public async Task<IActionResult> MarkRead(long id)
    {
        var n = await _db.Notifications.FirstOrDefaultAsync(n => n.Id == id && n.UserId == Uid);
        if (n == null) return NotFound();
        n.IsRead = true;
        await _db.SaveChangesAsync();
        return Ok(new MessageResponse("Marked as read"));
    }

    [HttpPut("read-all")]
    public async Task<IActionResult> MarkAllRead()
    {
        var items = await _db.Notifications.Where(n => n.UserId == Uid && !n.IsRead).ToListAsync();
        items.ForEach(n => n.IsRead = true);
        await _db.SaveChangesAsync();
        return Ok(new MessageResponse($"{items.Count} notifications marked as read"));
    }
}
