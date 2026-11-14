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
public class HelpdeskController : ControllerBase
{
    private readonly AppDbContext _db;
    private readonly NotificationService _notif;

    public HelpdeskController(AppDbContext db, NotificationService notif)
    {
        _db = db;
        _notif = notif;
    }

    private int GetEmployeeId() =>
        int.Parse(User.FindFirstValue("employeeId") ?? "0");

    [HttpPost]
    public async Task<IActionResult> Create([FromBody] CreateTicketRequest req)
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var count = await _db.HelpTickets.CountAsync() + 1;
        var ticket = new HelpTicket
        {
            EmployeeId = empId,
            TicketNo = $"TKT-{count:D6}",
            Category = req.Category,
            Subject = req.Subject,
            Description = req.Description,
            Priority = req.Priority
        };

        _db.HelpTickets.Add(ticket);
        await _db.SaveChangesAsync();

        return Ok(new { message = "Ticket created", ticketId = ticket.Id, ticketNo = ticket.TicketNo });
    }

    [HttpGet]
    public async Task<IActionResult> GetMyTickets([FromQuery] string? status)
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var query = _db.HelpTickets.Where(t => t.EmployeeId == empId);
        if (!string.IsNullOrEmpty(status))
            query = query.Where(t => t.Status == status);

        var tickets = await query.OrderByDescending(t => t.CreatedAt)
            .Select(t => new TicketDto
            {
                Id = t.Id,
                TicketNo = t.TicketNo,
                Category = t.Category,
                Subject = t.Subject,
                Description = t.Description,
                Priority = t.Priority,
                Status = t.Status,
                Resolution = t.Resolution,
                CreatedAt = t.CreatedAt,
                ResolvedAt = t.ResolvedAt
            })
            .ToListAsync();

        return Ok(tickets);
    }

    [HttpGet("{id}")]
    public async Task<IActionResult> GetDetail(long id)
    {
        var ticket = await _db.HelpTickets.Include(t => t.Comments)
            .FirstOrDefaultAsync(t => t.Id == id);
        if (ticket == null) return NotFound();

        var comments = ticket.Comments.OrderBy(c => c.CreatedAt)
            .Select(c => new TicketCommentDto(c.Id, "", c.Comment, c.CreatedAt))
            .ToList();

        return Ok(new TicketDto
        {
            Id = ticket.Id,
            TicketNo = ticket.TicketNo,
            Category = ticket.Category,
            Subject = ticket.Subject,
            Description = ticket.Description,
            Priority = ticket.Priority,
            Status = ticket.Status,
            Resolution = ticket.Resolution,
            CreatedAt = ticket.CreatedAt,
            ResolvedAt = ticket.ResolvedAt,
            Comments = comments
        });
    }

    [HttpPost("{id}/comment")]
    public async Task<IActionResult> AddComment(long id, [FromBody] AddCommentRequest req)
    {
        var ticket = await _db.HelpTickets.FindAsync(id);
        if (ticket == null) return NotFound();

        var userId = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);
        _db.TicketComments.Add(new TicketComment
        {
            TicketId = id,
            UserId = userId,
            Comment = req.Comment
        });
        await _db.SaveChangesAsync();

        return Ok(new { message = "Comment added" });
    }

    [Authorize(Roles = "admin,hr")]
    [HttpGet("all")]
    public async Task<IActionResult> GetAllTickets([FromQuery] string? status, [FromQuery] string? category)
    {
        var query = _db.HelpTickets.Include(t => t.Employee).AsQueryable();
        if (!string.IsNullOrEmpty(status))
            query = query.Where(t => t.Status == status);
        if (!string.IsNullOrEmpty(category))
            query = query.Where(t => t.Category == category);

        var tickets = await query.OrderByDescending(t => t.CreatedAt)
            .Select(t => new
            {
                t.Id, t.TicketNo, EmployeeName = t.Employee.FullName, t.Employee.EmpCode,
                t.Category, t.Subject, t.Priority, t.Status, t.CreatedAt, t.ResolvedAt
            })
            .ToListAsync();

        return Ok(tickets);
    }

    [Authorize(Roles = "admin,hr")]
    [HttpPut("{id}/status")]
    public async Task<IActionResult> UpdateStatus(long id, [FromBody] UpdateTicketStatusRequest req)
    {
        var ticket = await _db.HelpTickets.FindAsync(id);
        if (ticket == null) return NotFound();

        ticket.Status = req.Status;
        if (req.Status == "resolved")
        {
            ticket.ResolvedAt = DateTime.UtcNow;
            ticket.Resolution = req.Resolution;
        }
        if (req.Status == "closed")
            ticket.ClosedAt = DateTime.UtcNow;

        await _db.SaveChangesAsync();
        await _notif.SendAsync(ticket.EmployeeId, "Ticket Updated",
            $"Your ticket {ticket.TicketNo} status changed to {req.Status}",
            "helpdesk", ticket.Id);

        return Ok(new { message = "Status updated" });
    }
}

public record UpdateTicketStatusRequest(string Status, string? Resolution);
