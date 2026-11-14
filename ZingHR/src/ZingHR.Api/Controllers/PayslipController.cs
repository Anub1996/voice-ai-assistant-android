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
public class PayslipController : ControllerBase
{
    private readonly AppDbContext _db;

    public PayslipController(AppDbContext db) => _db = db;

    private int GetEmployeeId() =>
        int.Parse(User.FindFirstValue("employeeId") ?? "0");

    [HttpGet]
    public async Task<IActionResult> GetAll([FromQuery] int? year)
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var y = year ?? DateTime.UtcNow.Year;
        var payslips = await _db.Payslips
            .Where(p => p.EmployeeId == empId && p.Year == y && p.Status == "published")
            .OrderByDescending(p => p.Year).ThenByDescending(p => p.Month)
            .Select(p => new PayslipListDto(p.Id, p.PayPeriod, p.Month, p.Year, p.NetPay, p.Status, p.GeneratedAt))
            .ToListAsync();

        return Ok(payslips);
    }

    [HttpGet("{id}")]
    public async Task<IActionResult> GetDetail(long id)
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var p = await _db.Payslips.Include(ps => ps.Employee).ThenInclude(e => e.Department)
            .Include(ps => ps.Employee).ThenInclude(e => e.Designation)
            .FirstOrDefaultAsync(ps => ps.Id == id && ps.EmployeeId == empId);
        if (p == null) return NotFound();

        return Ok(new PayslipDetailDto
        {
            Id = p.Id,
            PayPeriod = p.PayPeriod,
            EmployeeName = p.Employee.FullName,
            EmpCode = p.Employee.EmpCode,
            Department = p.Employee.Department.Name,
            Designation = p.Employee.Designation.Title,
            Month = p.Month,
            Year = p.Year,
            BasicSalary = p.BasicSalary,
            Hra = p.Hra,
            ConveyanceAllowance = p.ConveyanceAllowance,
            MedicalAllowance = p.MedicalAllowance,
            SpecialAllowance = p.SpecialAllowance,
            OtherEarnings = p.OtherEarnings,
            GrossEarnings = p.GrossEarnings,
            Pf = p.Pf,
            Esi = p.Esi,
            ProfessionalTax = p.ProfessionalTax,
            Tds = p.Tds,
            LoanDeduction = p.LoanDeduction,
            OtherDeductions = p.OtherDeductions,
            TotalDeductions = p.TotalDeductions,
            NetPay = p.NetPay,
            PaidDays = p.PaidDays,
            LopDays = p.LopDays,
            Status = p.Status
        });
    }

    [HttpGet("tax-summary")]
    public async Task<IActionResult> TaxSummary([FromQuery] int? year)
    {
        var empId = GetEmployeeId();
        if (empId == 0) return Forbid();

        var fy = year ?? (DateTime.UtcNow.Month >= 4 ? DateTime.UtcNow.Year : DateTime.UtcNow.Year - 1);

        var payslips = await _db.Payslips
            .Where(p => p.EmployeeId == empId && p.Status == "published"
                && ((p.Year == fy && p.Month >= 4) || (p.Year == fy + 1 && p.Month <= 3)))
            .ToListAsync();

        return Ok(new
        {
            FinancialYear = $"{fy}-{fy + 1}",
            TotalGross = payslips.Sum(p => p.GrossEarnings),
            TotalPf = payslips.Sum(p => p.Pf),
            TotalEsi = payslips.Sum(p => p.Esi),
            TotalProfessionalTax = payslips.Sum(p => p.ProfessionalTax),
            TotalTds = payslips.Sum(p => p.Tds),
            TotalNet = payslips.Sum(p => p.NetPay),
            MonthlyBreakdown = payslips.OrderBy(p => p.Year).ThenBy(p => p.Month)
                .Select(p => new { p.PayPeriod, p.GrossEarnings, p.TotalDeductions, p.NetPay, p.Tds })
        });
    }

    [Authorize(Roles = "admin,hr")]
    [HttpPost("generate")]
    public async Task<IActionResult> GeneratePayslips([FromBody] GeneratePayslipRequest req)
    {
        var employees = await _db.Employees.Where(e => e.IsActive)
            .Include(e => e.Shift).ToListAsync();

        var structures = await _db.SalaryStructures.Where(s => s.IsCurrent).ToListAsync();
        int generated = 0;

        foreach (var emp in employees)
        {
            var existing = await _db.Payslips.AnyAsync(p => p.EmployeeId == emp.Id && p.Month == req.Month && p.Year == req.Year);
            if (existing) continue;

            var salary = structures.FirstOrDefault(s => s.EmployeeId == emp.Id);
            if (salary == null) continue;

            var start = new DateOnly(req.Year, req.Month, 1);
            var end = start.AddMonths(1);
            var lop = await _db.Attendances.CountAsync(a =>
                a.EmployeeId == emp.Id && a.Date >= start && a.Date < end && a.Status == "absent");
            int totalDays = DateTime.DaysInMonth(req.Year, req.Month);
            int paidDays = totalDays - lop;

            decimal ratio = (decimal)paidDays / totalDays;

            var payslip = new Payslip
            {
                EmployeeId = emp.Id,
                Month = req.Month,
                Year = req.Year,
                PayPeriod = $"{start:MMM yyyy}",
                BasicSalary = Math.Round(salary.BasicSalary * ratio, 0),
                Hra = Math.Round(salary.Hra * ratio, 0),
                ConveyanceAllowance = Math.Round(salary.ConveyanceAllowance * ratio, 0),
                MedicalAllowance = Math.Round(salary.MedicalAllowance * ratio, 0),
                SpecialAllowance = Math.Round(salary.SpecialAllowance * ratio, 0),
                Pf = Math.Round(salary.Pf * ratio, 0),
                Esi = Math.Round(salary.Esi * ratio, 0),
                ProfessionalTax = salary.ProfessionalTax,
                Tds = Math.Round(salary.Tds * ratio, 0),
                PaidDays = paidDays,
                LopDays = lop,
                PresentDays = paidDays,
                Status = "draft"
            };

            payslip.GrossEarnings = payslip.BasicSalary + payslip.Hra + payslip.ConveyanceAllowance
                + payslip.MedicalAllowance + payslip.SpecialAllowance + payslip.OtherEarnings;
            payslip.TotalDeductions = payslip.Pf + payslip.Esi + payslip.ProfessionalTax
                + payslip.Tds + payslip.LoanDeduction + payslip.OtherDeductions;
            payslip.NetPay = payslip.GrossEarnings - payslip.TotalDeductions;

            _db.Payslips.Add(payslip);
            generated++;
        }

        await _db.SaveChangesAsync();
        return Ok(new { message = $"{generated} payslips generated", month = req.Month, year = req.Year });
    }

    [Authorize(Roles = "admin,hr")]
    [HttpPut("publish")]
    public async Task<IActionResult> PublishPayslips([FromBody] GeneratePayslipRequest req)
    {
        var payslips = await _db.Payslips
            .Where(p => p.Month == req.Month && p.Year == req.Year && p.Status == "draft")
            .ToListAsync();

        foreach (var p in payslips)
        {
            p.Status = "published";
            p.PublishedAt = DateTime.UtcNow;
        }

        await _db.SaveChangesAsync();
        return Ok(new { message = $"{payslips.Count} payslips published" });
    }
}

public record GeneratePayslipRequest(int Month, int Year);
