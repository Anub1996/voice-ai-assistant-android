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
public class LeaveController : ControllerBase
{
    private readonly AppDbContext _db;
    private readonly NotificationService _notif;

    public LeaveController(AppDbContext db, NotificationService notif)
    {
        _db = db;
        _notif = notif;
    }

    private int GetEmployeeId() =>
        int.Parse(User.FindFirstValue("employeeId") ?? "0");

    [HttpGet("types")]
    public async Task<IActionResult> GetLeaveTypes()
    {
        var types = await _db.LeaveTypes.Where(t => t.IsActive)
            .Select(t => new { t.Id, t.Name, t.Code, t.Color, t.DefaultDays, t.IsPaid, t.AllowHalfDay })
            .ToListAsync();
        return Ok(types);
    }

    [HttpGet("balance")]
    public async Task<IActionResult> GetBalance([FromQuery] int? year)
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var y = year ?? DateTime.UtcNow.Year;
        var balances = await _db.LeaveBalances
            .Include(b => b.LeaveType)
            .Where(b => b.EmployeeId == empId && b.Year == y)
            .Select(b => new LeaveBalanceDto(b.LeaveTypeId, b.LeaveType.Name, b.LeaveType.Color, b.Entitled, b.Used, b.Balance, b.LeaveType.IsPaid))
            .ToListAsync();

        return Ok(balances);
    }

    [HttpPost("apply")]
    public async Task<IActionResult> Apply([FromBody] LeaveApplyRequest req)
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var leaveType = await _db.LeaveTypes.FindAsync(req.LeaveTypeId);
        if (leaveType == null) return BadRequest(new { message = "Invalid leave type" });

        double totalDays = req.IsHalfDay ? 0.5 : (req.ToDate.DayNumber - req.FromDate.DayNumber + 1);

        var balance = await _db.LeaveBalances
            .FirstOrDefaultAsync(b => b.EmployeeId == empId && b.LeaveTypeId == req.LeaveTypeId && b.Year == req.FromDate.Year);

        if (balance != null && balance.Balance < totalDays && leaveType.Code != "LOP")
            return BadRequest(new { message = $"Insufficient leave balance. Available: {balance.Balance} days" });

        var overlap = await _db.LeaveRequests
            .AnyAsync(l => l.EmployeeId == empId && l.Status != "cancelled" && l.Status != "rejected"
                && l.FromDate <= req.ToDate && l.ToDate >= req.FromDate);
        if (overlap)
            return BadRequest(new { message = "Overlapping leave request exists" });

        var leave = new LeaveRequest
        {
            EmployeeId = empId,
            LeaveTypeId = req.LeaveTypeId,
            FromDate = req.FromDate,
            ToDate = req.ToDate,
            TotalDays = totalDays,
            IsHalfDay = req.IsHalfDay,
            HalfDayType = req.HalfDayType,
            Reason = req.Reason,
            EmergencyContact = req.EmergencyContact
        };

        _db.LeaveRequests.Add(leave);
        await _db.SaveChangesAsync();

        var emp = await _db.Employees.FindAsync(empId);
        await _notif.SendToManagerAsync(empId,
            "Leave Request",
            $"{emp?.FullName} has applied for {leaveType.Name} ({req.FromDate} to {req.ToDate})",
            "leave", leave.Id);

        return Ok(new { message = "Leave applied", leaveId = leave.Id });
    }

    [HttpGet("history")]
    public async Task<IActionResult> History([FromQuery] int? year)
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var y = year ?? DateTime.UtcNow.Year;
        var start = new DateOnly(y, 1, 1);
        var end = start.AddYears(1);

        var requests = await _db.LeaveRequests
            .Include(l => l.LeaveType)
            .Where(l => l.EmployeeId == empId && l.FromDate >= start && l.FromDate < end)
            .OrderByDescending(l => l.AppliedAt)
            .Select(l => new LeaveRequestDto
            {
                Id = l.Id,
                LeaveType = l.LeaveType.Name,
                LeaveTypeColor = l.LeaveType.Color,
                FromDate = l.FromDate,
                ToDate = l.ToDate,
                TotalDays = l.TotalDays,
                IsHalfDay = l.IsHalfDay,
                Reason = l.Reason,
                Status = l.Status,
                ApproverRemarks = l.ApproverRemarks,
                AppliedAt = l.AppliedAt,
                ActionDate = l.ActionDate
            })
            .ToListAsync();

        return Ok(requests);
    }

    [HttpDelete("{id}")]
    public async Task<IActionResult> Cancel(long id)
    {
        var empId = GetEmployeeId();
        var leave = await _db.LeaveRequests.FirstOrDefaultAsync(l => l.Id == id && l.EmployeeId == empId);
        if (leave == null) return NotFound();
        if (leave.Status != "pending")
            return BadRequest(new { message = "Can only cancel pending requests" });

        leave.Status = "cancelled";
        await _db.SaveChangesAsync();
        return Ok(new { message = "Leave cancelled" });
    }

    [Authorize(Roles = "admin,hr,manager")]
    [HttpGet("pending")]
    public async Task<IActionResult> PendingApprovals()
    {
        var empId = GetEmployeeId();
        var subordinateIds = await _db.Employees
            .Where(e => e.ReportingManagerId == empId)
            .Select(e => e.Id)
            .ToListAsync();

        var role = User.FindFirstValue(ClaimTypes.Role);
        IQueryable<LeaveRequest> query = _db.LeaveRequests.Include(l => l.LeaveType).Include(l => l.Employee);

        if (role is "admin" or "hr")
            query = query.Where(l => l.Status == "pending");
        else
            query = query.Where(l => l.Status == "pending" && subordinateIds.Contains(l.EmployeeId));

        var requests = await query.OrderBy(l => l.AppliedAt)
            .Select(l => new
            {
                l.Id,
                EmployeeName = l.Employee.FullName,
                l.Employee.EmpCode,
                LeaveType = l.LeaveType.Name,
                l.FromDate, l.ToDate, l.TotalDays, l.IsHalfDay, l.Reason, l.AppliedAt
            })
            .ToListAsync();

        return Ok(requests);
    }

    [Authorize(Roles = "admin,hr,manager")]
    [HttpPut("{id}/action")]
    public async Task<IActionResult> Action(long id, [FromBody] LeaveActionRequest req)
    {
        var leave = await _db.LeaveRequests.Include(l => l.LeaveType).FirstOrDefaultAsync(l => l.Id == id);
        if (leave == null) return NotFound();
        if (leave.Status != "pending")
            return BadRequest(new { message = "Leave is not pending" });

        var userId = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);

        if (req.Action == "approve")
        {
            leave.Status = "approved";
            leave.ApprovedBy = userId;
            leave.ActionDate = DateTime.UtcNow;
            leave.ApproverRemarks = req.Remarks;

            var balance = await _db.LeaveBalances
                .FirstOrDefaultAsync(b => b.EmployeeId == leave.EmployeeId && b.LeaveTypeId == leave.LeaveTypeId && b.Year == leave.FromDate.Year);
            if (balance != null)
            {
                balance.Used += leave.TotalDays;
                balance.Balance = balance.Entitled + balance.CarriedForward - balance.Used;
            }

            for (var d = leave.FromDate; d <= leave.ToDate; d = d.AddDays(1))
            {
                var att = await _db.Attendances.FirstOrDefaultAsync(a => a.EmployeeId == leave.EmployeeId && a.Date == d);
                if (att == null)
                {
                    _db.Attendances.Add(new Attendance
                    {
                        EmployeeId = leave.EmployeeId,
                        Date = d,
                        Status = leave.IsHalfDay ? "half-day" : "on-leave"
                    });
                }
                else
                {
                    att.Status = leave.IsHalfDay ? "half-day" : "on-leave";
                }
            }

            await _notif.SendAsync(leave.EmployeeId, "Leave Approved",
                $"Your {leave.LeaveType.Name} from {leave.FromDate} to {leave.ToDate} has been approved.",
                "leave", leave.Id);
        }
        else if (req.Action == "reject")
        {
            leave.Status = "rejected";
            leave.ApprovedBy = userId;
            leave.ActionDate = DateTime.UtcNow;
            leave.ApproverRemarks = req.Remarks;

            await _notif.SendAsync(leave.EmployeeId, "Leave Rejected",
                $"Your {leave.LeaveType.Name} from {leave.FromDate} to {leave.ToDate} has been rejected. Reason: {req.Remarks}",
                "leave", leave.Id);
        }
        else
        {
            return BadRequest(new { message = "Action must be 'approve' or 'reject'" });
        }

        await _db.SaveChangesAsync();
        return Ok(new { message = $"Leave {leave.Status}" });
    }
}
