using Microsoft.AspNetCore.Mvc;
using Microsoft.AspNetCore.SignalR;
using Microsoft.EntityFrameworkCore;
using WorkSentinel.Server.Data;
using WorkSentinel.Server.Hubs;
using WorkSentinel.Server.Models;
using WorkSentinel.Server.Services;

namespace WorkSentinel.Server.Controllers;

[ApiController]
[Route("api/[controller]")]
public class AgentController : ControllerBase
{
    private readonly AppDbContext _db;
    private readonly AlertService _alerts;
    private readonly ProductivityService _productivity;
    private readonly IHubContext<MonitorHub> _hub;

    public AgentController(AppDbContext db, AlertService alerts, ProductivityService productivity, IHubContext<MonitorHub> hub)
    {
        _db = db;
        _alerts = alerts;
        _productivity = productivity;
        _hub = hub;
    }

    [HttpPost("register")]
    public async Task<IActionResult> Register([FromBody] AgentRegisterRequest req)
    {
        var emp = await _db.Employees.FirstOrDefaultAsync(e => e.EmployeeId == req.EmployeeId);
        if (emp == null)
        {
            emp = new Employee
            {
                EmployeeId = req.EmployeeId,
                FullName = req.FullName,
                Email = req.Email,
                Department = req.Department,
                ComputerName = req.ComputerName,
                Status = "online",
                LastSeenAt = DateTime.UtcNow
            };
            _db.Employees.Add(emp);
            await _db.SaveChangesAsync();
        }
        else
        {
            emp.ComputerName = req.ComputerName;
            emp.Status = "online";
            emp.LastSeenAt = DateTime.UtcNow;
            await _db.SaveChangesAsync();
        }

        await _hub.Clients.Group("dashboard").SendAsync("EmployeeStatusChanged", emp.Id, "online");
        return Ok(new { employeeId = emp.Id, message = "Registered" });
    }

    [HttpPost("heartbeat")]
    public async Task<IActionResult> Heartbeat([FromBody] AgentHeartbeatRequest req)
    {
        var emp = await _db.Employees.FirstOrDefaultAsync(e => e.EmployeeId == req.EmployeeId);
        if (emp == null) return NotFound();

        emp.Status = req.Status;
        emp.LastSeenAt = DateTime.UtcNow;
        await _db.SaveChangesAsync();

        await _hub.Clients.Group("dashboard").SendAsync("EmployeeStatusChanged", emp.Id, req.Status);
        await _hub.Clients.Group($"employee-{emp.Id}").SendAsync("Heartbeat", new { req.ActiveWindow, req.ActiveApp, req.Status });
        return Ok();
    }

    [HttpPost("data")]
    public async Task<IActionResult> IngestData([FromBody] AgentDataPayload payload)
    {
        var emp = await _db.Employees.FirstOrDefaultAsync(e => e.EmployeeId == payload.EmployeeId);
        if (emp == null) return NotFound();

        // Activities
        foreach (var a in payload.Activities)
        {
            _db.ActivityLogs.Add(new ActivityLog
            {
                EmployeeId = emp.Id,
                ActivityType = a.ActivityType,
                StartTime = a.StartTime,
                EndTime = a.EndTime,
                DurationSeconds = a.DurationSeconds,
                ActiveWindow = a.ActiveWindow,
                ActiveApp = a.ActiveApp,
                KeystrokeCount = a.KeystrokeCount,
                MouseClickCount = a.MouseClickCount
            });
        }

        // App usage
        foreach (var a in payload.AppUsages)
        {
            var category = await _productivity.CategorizeAppAsync(a.AppName);
            _db.AppUsages.Add(new AppUsage
            {
                EmployeeId = emp.Id,
                AppName = a.AppName,
                WindowTitle = a.WindowTitle,
                ProcessName = a.ProcessName,
                Category = category,
                StartTime = a.StartTime,
                DurationSeconds = a.DurationSeconds,
                IsFocused = a.IsFocused
            });
        }

        // Web visits
        foreach (var w in payload.WebVisits)
        {
            var category = await _productivity.CategorizeWebAsync(w.Domain);
            var visit = new WebVisit
            {
                EmployeeId = emp.Id,
                Url = w.Url,
                Title = w.Title,
                Domain = w.Domain,
                Category = category,
                Browser = w.Browser,
                VisitTime = w.VisitTime,
                DurationSeconds = w.DurationSeconds
            };
            _db.WebVisits.Add(visit);
            await _alerts.EvaluateWebVisitAsync(visit, emp.Id);
        }

        // Emails
        foreach (var e in payload.Emails)
        {
            var email = new EmailLog
            {
                EmployeeId = emp.Id,
                Sender = e.Sender,
                Recipients = e.Recipients,
                Cc = e.Cc,
                Subject = e.Subject,
                BodyPreview = e.BodyPreview,
                Direction = e.Direction,
                EmailClient = e.EmailClient,
                HasAttachment = e.HasAttachment,
                AttachmentNames = e.AttachmentNames,
                Timestamp = e.Timestamp
            };
            _db.EmailLogs.Add(email);
            await _alerts.EvaluateEmailAsync(email, emp.Id);
        }

        // File activities
        foreach (var f in payload.Files)
        {
            var file = new FileActivity
            {
                EmployeeId = emp.Id,
                FilePath = f.FilePath,
                FileName = f.FileName,
                Action = f.Action,
                Destination = f.Destination,
                FileSizeBytes = f.FileSizeBytes,
                Timestamp = f.Timestamp
            };
            _db.FileActivities.Add(file);
            await _alerts.EvaluateFileActivityAsync(file, emp.Id);
        }

        await _db.SaveChangesAsync();

        // Recalculate today's productivity
        await _productivity.RecalculateDailyScoreAsync(emp.Id, DateOnly.FromDateTime(DateTime.UtcNow));

        // Notify dashboard
        await _hub.Clients.Group("dashboard").SendAsync("DataIngested", emp.Id, emp.FullName);

        return Ok(new { processed = true });
    }

    [HttpPost("screenshot")]
    [RequestSizeLimit(10_000_000)]
    public async Task<IActionResult> UploadScreenshot([FromForm] string employeeId, IFormFile file)
    {
        var emp = await _db.Employees.FirstOrDefaultAsync(e => e.EmployeeId == employeeId);
        if (emp == null) return NotFound();

        var dir = Path.Combine("screenshots", emp.EmployeeId, DateTime.UtcNow.ToString("yyyy-MM-dd"));
        Directory.CreateDirectory(dir);
        var filename = $"{DateTime.UtcNow:HHmmss}_{Guid.NewGuid():N}.png";
        var filepath = Path.Combine(dir, filename);

        using (var stream = new FileStream(filepath, FileMode.Create))
            await file.CopyToAsync(stream);

        _db.ScreenCaptures.Add(new ScreenCapture
        {
            EmployeeId = emp.Id,
            FilePath = filepath,
            FileSizeBytes = file.Length,
            CapturedAt = DateTime.UtcNow
        });
        await _db.SaveChangesAsync();

        await _hub.Clients.Group($"employee-{emp.Id}").SendAsync("NewScreenshot", filepath);
        return Ok();
    }
}
