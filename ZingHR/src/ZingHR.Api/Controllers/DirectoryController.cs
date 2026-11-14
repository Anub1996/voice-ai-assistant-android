using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using ZingHR.Api.Data;
using ZingHR.Api.Models;

namespace ZingHR.Api.Controllers;

[ApiController]
[Route("api/[controller]")]
[Authorize]
public class DirectoryController : ControllerBase
{
    private readonly AppDbContext _db;

    public DirectoryController(AppDbContext db) => _db = db;

    [HttpGet]
    public async Task<IActionResult> GetAll([FromQuery] string? search, [FromQuery] int? departmentId)
    {
        var query = _db.Employees.Include(e => e.Department).Include(e => e.Designation)
            .Where(e => e.IsActive);

        if (!string.IsNullOrWhiteSpace(search))
            query = query.Where(e => e.FullName.Contains(search) || e.Email.Contains(search) || e.EmpCode.Contains(search) || e.Phone.Contains(search));

        if (departmentId.HasValue)
            query = query.Where(e => e.DepartmentId == departmentId.Value);

        var employees = await query.OrderBy(e => e.FullName)
            .Select(e => new DirectoryEntryDto(
                e.Id, e.FullName, e.EmpCode, e.Department.Name, e.Designation.Title,
                e.Email, e.Phone, e.ProfilePhotoUrl))
            .ToListAsync();

        return Ok(employees);
    }

    [HttpGet("{id}")]
    public async Task<IActionResult> GetProfile(int id)
    {
        var emp = await _db.Employees.Include(e => e.Department).Include(e => e.Designation)
            .Include(e => e.ReportingManager)
            .FirstOrDefaultAsync(e => e.Id == id && e.IsActive);
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

    [HttpGet("departments")]
    public async Task<IActionResult> GetDepartments()
    {
        var depts = await _db.Departments.Where(d => d.IsActive)
            .Select(d => new { d.Id, d.Name, d.Code, EmployeeCount = d.Employees.Count(e => e.IsActive) })
            .ToListAsync();
        return Ok(depts);
    }

    [HttpGet("org-chart")]
    public async Task<IActionResult> OrgChart()
    {
        var employees = await _db.Employees
            .Where(e => e.IsActive)
            .Include(e => e.Department).Include(e => e.Designation)
            .Select(e => new
            {
                e.Id,
                e.FullName,
                e.EmpCode,
                Department = e.Department.Name,
                Designation = e.Designation.Title,
                e.ProfilePhotoUrl,
                e.ReportingManagerId
            })
            .ToListAsync();
        return Ok(employees);
    }
}
