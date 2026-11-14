using ZingHR.Api.Data;
using ZingHR.Api.Models;

namespace ZingHR.Api.Services;

public class NotificationService
{
    private readonly AppDbContext _db;

    public NotificationService(AppDbContext db) => _db = db;

    public async Task SendAsync(int employeeId, string title, string body, string type, long? referenceId = null)
    {
        _db.Notifications.Add(new Notification
        {
            EmployeeId = employeeId,
            Title = title,
            Body = body,
            Type = type,
            ReferenceId = referenceId
        });
        await _db.SaveChangesAsync();
    }

    public async Task SendToManagerAsync(int employeeId, string title, string body, string type, long? referenceId = null)
    {
        var emp = await _db.Employees.FindAsync(employeeId);
        if (emp?.ReportingManagerId != null)
        {
            await SendAsync(emp.ReportingManagerId.Value, title, body, type, referenceId);
        }
    }
}
