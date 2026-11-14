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
public class ExpenseController : ControllerBase
{
    private readonly AppDbContext _db;
    private readonly NotificationService _notif;

    public ExpenseController(AppDbContext db, NotificationService notif)
    {
        _db = db;
        _notif = notif;
    }

    private int GetEmployeeId() =>
        int.Parse(User.FindFirstValue("employeeId") ?? "0");

    [HttpGet("categories")]
    public async Task<IActionResult> GetCategories()
    {
        var cats = await _db.ExpenseCategories.Where(c => c.IsActive)
            .Select(c => new { c.Id, c.Name, c.MaxLimit, c.RequiresReceipt })
            .ToListAsync();
        return Ok(cats);
    }

    [HttpPost]
    public async Task<IActionResult> Submit([FromBody] ExpenseSubmitRequest req)
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var category = await _db.ExpenseCategories.FindAsync(req.CategoryId);
        if (category == null) return BadRequest(new { message = "Invalid category" });
        if (category.MaxLimit.HasValue && req.Amount > category.MaxLimit.Value)
            return BadRequest(new { message = $"Amount exceeds limit of {category.MaxLimit}" });

        var expense = new Expense
        {
            EmployeeId = empId,
            CategoryId = req.CategoryId,
            Title = req.Title,
            Description = req.Description ?? "",
            Amount = req.Amount,
            ExpenseDate = req.ExpenseDate,
            ReceiptUrl = req.ReceiptUrl
        };

        _db.Expenses.Add(expense);
        await _db.SaveChangesAsync();

        var emp = await _db.Employees.FindAsync(empId);
        await _notif.SendToManagerAsync(empId,
            "Expense Claim",
            $"{emp?.FullName} submitted expense '{req.Title}' for ₹{req.Amount}",
            "expense", expense.Id);

        return Ok(new { message = "Expense submitted", expenseId = expense.Id });
    }

    [HttpGet]
    public async Task<IActionResult> GetAll([FromQuery] string? status, [FromQuery] int? year)
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var query = _db.Expenses.Include(e => e.Category)
            .Where(e => e.EmployeeId == empId);

        if (!string.IsNullOrEmpty(status))
            query = query.Where(e => e.Status == status);
        if (year.HasValue)
            query = query.Where(e => e.ExpenseDate.Year == year.Value);

        var expenses = await query.OrderByDescending(e => e.SubmittedAt)
            .Select(e => new ExpenseDto
            {
                Id = e.Id,
                Category = e.Category.Name,
                Title = e.Title,
                Amount = e.Amount,
                ExpenseDate = e.ExpenseDate,
                Status = e.Status,
                ApprovedAmount = e.ApprovedAmount,
                ApproverRemarks = e.ApproverRemarks,
                SubmittedAt = e.SubmittedAt
            })
            .ToListAsync();

        return Ok(expenses);
    }

    [Authorize(Roles = "admin,hr,manager")]
    [HttpGet("pending")]
    public async Task<IActionResult> PendingApprovals()
    {
        var empId = GetEmployeeId();
        var role = User.FindFirstValue(ClaimTypes.Role);

        IQueryable<Expense> query = _db.Expenses.Include(e => e.Category).Include(e => e.Employee);

        if (role is "admin" or "hr")
            query = query.Where(e => e.Status == "pending");
        else
        {
            var subIds = await _db.Employees.Where(e => e.ReportingManagerId == empId)
                .Select(e => e.Id).ToListAsync();
            query = query.Where(e => e.Status == "pending" && subIds.Contains(e.EmployeeId));
        }

        var expenses = await query.OrderBy(e => e.SubmittedAt)
            .Select(e => new
            {
                e.Id,
                EmployeeName = e.Employee.FullName,
                e.Employee.EmpCode,
                Category = e.Category.Name,
                e.Title, e.Amount, e.ExpenseDate, e.ReceiptUrl, e.SubmittedAt
            })
            .ToListAsync();

        return Ok(expenses);
    }

    [Authorize(Roles = "admin,hr,manager")]
    [HttpPut("{id}/action")]
    public async Task<IActionResult> Action(long id, [FromBody] ExpenseActionRequest req)
    {
        var expense = await _db.Expenses.Include(e => e.Category).FirstOrDefaultAsync(e => e.Id == id);
        if (expense == null) return NotFound();
        if (expense.Status != "pending")
            return BadRequest(new { message = "Expense is not pending" });

        var userId = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);

        if (req.Action == "approve")
        {
            expense.Status = "approved";
            expense.ApprovedBy = userId;
            expense.ActionDate = DateTime.UtcNow;
            expense.ApprovedAmount = req.ApprovedAmount ?? expense.Amount;
            expense.ApproverRemarks = req.Remarks;

            await _notif.SendAsync(expense.EmployeeId, "Expense Approved",
                $"Your expense '{expense.Title}' for ₹{expense.Amount} has been approved.",
                "expense", expense.Id);
        }
        else if (req.Action == "reject")
        {
            expense.Status = "rejected";
            expense.ApprovedBy = userId;
            expense.ActionDate = DateTime.UtcNow;
            expense.ApproverRemarks = req.Remarks;

            await _notif.SendAsync(expense.EmployeeId, "Expense Rejected",
                $"Your expense '{expense.Title}' has been rejected. Reason: {req.Remarks}",
                "expense", expense.Id);
        }

        await _db.SaveChangesAsync();
        return Ok(new { message = $"Expense {expense.Status}" });
    }
}

public record ExpenseActionRequest(string Action, decimal? ApprovedAmount, string? Remarks);
