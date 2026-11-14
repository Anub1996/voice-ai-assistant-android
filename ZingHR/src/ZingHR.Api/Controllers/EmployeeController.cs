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
public class EmployeeController : ControllerBase
{
    private readonly AppDbContext _db;

    public EmployeeController(AppDbContext db) => _db = db;

    private int GetEmployeeId() =>
        int.Parse(User.FindFirstValue("employeeId") ?? "0");

    [HttpGet("profile")]
    public async Task<IActionResult> MyProfile()
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var emp = await _db.Employees.Include(e => e.Department).Include(e => e.Designation)
            .Include(e => e.ReportingManager)
            .FirstOrDefaultAsync(e => e.Id == empId);
        if (emp == null) return NotFound();

        return Ok(new ProfileDto
        {
            Id = emp.Id,
            EmpCode = emp.EmpCode,
            FullName = emp.FullName,
            Email = emp.Email,
            Phone = emp.Phone,
            Gender = emp.Gender,
            Department = emp.Department.Name,
            Designation = emp.Designation.Title,
            ReportingManager = emp.ReportingManager?.FullName ?? "",
            ProfilePhotoUrl = emp.ProfilePhotoUrl,
            DateOfJoining = emp.DateOfJoining,
            EmploymentType = emp.EmploymentType,
            Status = emp.Status
        });
    }

    [HttpPut("profile")]
    public async Task<IActionResult> UpdateProfile([FromBody] ProfileUpdateRequest req)
    {
        var empId = GetEmployeeId();
        var emp = await _db.Employees.FindAsync(empId);
        if (emp == null) return NotFound();

        if (req.Phone != null) emp.Phone = req.Phone;
        if (req.Address != null) emp.Address = req.Address;
        if (req.City != null) emp.City = req.City;
        if (req.State != null) emp.State = req.State;
        if (req.PinCode != null) emp.PinCode = req.PinCode;
        emp.UpdatedAt = DateTime.UtcNow;

        await _db.SaveChangesAsync();
        return Ok(new { message = "Profile updated" });
    }

    [HttpPost("profile-photo")]
    [RequestSizeLimit(5_000_000)]
    public async Task<IActionResult> UploadPhoto(IFormFile file)
    {
        var empId = GetEmployeeId();
        var emp = await _db.Employees.FindAsync(empId);
        if (emp == null) return NotFound();

        var dir = Path.Combine("uploads", "photos");
        Directory.CreateDirectory(dir);
        var filename = $"{emp.EmpCode}_{Guid.NewGuid():N}{Path.GetExtension(file.FileName)}";
        var filepath = Path.Combine(dir, filename);

        using (var stream = new FileStream(filepath, FileMode.Create))
            await file.CopyToAsync(stream);

        emp.ProfilePhotoUrl = $"/uploads/photos/{filename}";
        emp.UpdatedAt = DateTime.UtcNow;
        await _db.SaveChangesAsync();

        return Ok(new { message = "Photo uploaded", url = emp.ProfilePhotoUrl });
    }

    [Authorize(Roles = "admin,hr")]
    [HttpGet]
    public async Task<IActionResult> GetAll([FromQuery] string? search, [FromQuery] int? departmentId, [FromQuery] string? status)
    {
        var query = _db.Employees.Include(e => e.Department).Include(e => e.Designation).AsQueryable();

        if (!string.IsNullOrWhiteSpace(search))
            query = query.Where(e => e.FullName.Contains(search) || e.EmpCode.Contains(search) || e.Email.Contains(search));
        if (departmentId.HasValue)
            query = query.Where(e => e.DepartmentId == departmentId.Value);
        if (!string.IsNullOrWhiteSpace(status))
            query = query.Where(e => e.Status == status);

        var employees = await query.OrderBy(e => e.FullName)
            .Select(e => new
            {
                e.Id, e.EmpCode, e.FullName, e.Email, e.Phone,
                Department = e.Department.Name, Designation = e.Designation.Title,
                e.DateOfJoining, e.EmploymentType, e.Status, e.IsActive
            })
            .ToListAsync();

        return Ok(employees);
    }

    [Authorize(Roles = "admin,hr")]
    [HttpPost]
    public async Task<IActionResult> Create([FromBody] CreateEmployeeRequest req)
    {
        if (await _db.Employees.AnyAsync(e => e.EmpCode == req.EmpCode))
            return BadRequest(new { message = "Employee code already exists" });

        var emp = new Employee
        {
            EmpCode = req.EmpCode,
            FullName = $"{req.FirstName} {req.LastName}".Trim(),
            FirstName = req.FirstName,
            LastName = req.LastName,
            Email = req.Email,
            Phone = req.Phone ?? "",
            Gender = req.Gender ?? "",
            DateOfJoining = req.DateOfJoining,
            DepartmentId = req.DepartmentId,
            DesignationId = req.DesignationId,
            ReportingManagerId = req.ReportingManagerId,
            ShiftId = req.ShiftId ?? 1,
            EmploymentType = req.EmploymentType ?? "full-time"
        };

        _db.Employees.Add(emp);
        await _db.SaveChangesAsync();

        var year = DateTime.UtcNow.Year;
        var leaveTypes = await _db.LeaveTypes.Where(t => t.IsActive).ToListAsync();
        foreach (var lt in leaveTypes)
        {
            _db.LeaveBalances.Add(new LeaveBalance
            {
                EmployeeId = emp.Id,
                LeaveTypeId = lt.Id,
                Year = year,
                Entitled = lt.DefaultDays,
                Balance = lt.DefaultDays
            });
        }

        if (!string.IsNullOrEmpty(req.Password))
        {
            _db.Users.Add(new AppUser
            {
                Username = req.EmpCode.ToLowerInvariant(),
                PasswordHash = BCrypt.Net.BCrypt.HashPassword(req.Password),
                Role = "employee",
                EmployeeId = emp.Id,
                FullName = emp.FullName,
                Email = emp.Email
            });
        }

        await _db.SaveChangesAsync();
        return Ok(new { message = "Employee created", employeeId = emp.Id });
    }
}

public record CreateEmployeeRequest(
    string EmpCode, string FirstName, string LastName, string Email, string? Phone,
    string? Gender, DateOnly DateOfJoining, int DepartmentId, int DesignationId,
    int? ReportingManagerId, int? ShiftId, string? EmploymentType, string? Password);
