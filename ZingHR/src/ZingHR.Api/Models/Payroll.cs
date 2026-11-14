using System.ComponentModel.DataAnnotations;

namespace ZingHR.Api.Models;

public class SalaryStructure
{
    public int Id { get; set; }
    public int EmployeeId { get; set; }

    public decimal BasicSalary { get; set; }
    public decimal Hra { get; set; }
    public decimal ConveyanceAllowance { get; set; }
    public decimal MedicalAllowance { get; set; }
    public decimal SpecialAllowance { get; set; }
    public decimal OtherAllowances { get; set; }

    public decimal GrossSalary { get; set; }

    public decimal Pf { get; set; }
    public decimal Esi { get; set; }
    public decimal ProfessionalTax { get; set; }
    public decimal Tds { get; set; }
    public decimal OtherDeductions { get; set; }

    public decimal NetSalary { get; set; }

    public DateOnly EffectiveFrom { get; set; }
    public DateOnly? EffectiveTo { get; set; }
    public bool IsCurrent { get; set; } = true;

    public Employee Employee { get; set; } = null!;
}

public class Payslip
{
    public long Id { get; set; }
    public int EmployeeId { get; set; }

    public int Month { get; set; }
    public int Year { get; set; }

    [MaxLength(20)]
    public string PayPeriod { get; set; } = "";

    public decimal BasicSalary { get; set; }
    public decimal Hra { get; set; }
    public decimal ConveyanceAllowance { get; set; }
    public decimal MedicalAllowance { get; set; }
    public decimal SpecialAllowance { get; set; }
    public decimal OtherEarnings { get; set; }
    public decimal GrossEarnings { get; set; }

    public decimal Pf { get; set; }
    public decimal Esi { get; set; }
    public decimal ProfessionalTax { get; set; }
    public decimal Tds { get; set; }
    public decimal LoanDeduction { get; set; }
    public decimal OtherDeductions { get; set; }
    public decimal TotalDeductions { get; set; }

    public decimal NetPay { get; set; }

    public int PaidDays { get; set; }
    public int LopDays { get; set; }
    public int PresentDays { get; set; }
    public int LeaveDays { get; set; }
    public int HolidayDays { get; set; }
    public int WeekOffDays { get; set; }

    [MaxLength(50)]
    public string Status { get; set; } = "draft"; // draft, generated, published

    public DateTime GeneratedAt { get; set; } = DateTime.UtcNow;
    public DateTime? PublishedAt { get; set; }

    public Employee Employee { get; set; } = null!;
}
