using System.ComponentModel.DataAnnotations;

namespace ZingHR.Api.Models;

public class ExpenseCategory
{
    public int Id { get; set; }

    [Required, MaxLength(100)]
    public string Name { get; set; } = "";

    public decimal? MaxLimit { get; set; }
    public bool RequiresReceipt { get; set; } = true;
    public bool IsActive { get; set; } = true;
}

public class Expense
{
    public long Id { get; set; }
    public int EmployeeId { get; set; }
    public int CategoryId { get; set; }

    [MaxLength(300)]
    public string Title { get; set; } = "";

    [MaxLength(1000)]
    public string Description { get; set; } = "";

    public decimal Amount { get; set; }

    [MaxLength(10)]
    public string Currency { get; set; } = "INR";

    public DateOnly ExpenseDate { get; set; }

    [MaxLength(500)]
    public string? ReceiptUrl { get; set; }

    [MaxLength(50)]
    public string Status { get; set; } = "pending"; // pending, approved, rejected, reimbursed

    public int? ApprovedBy { get; set; }
    public DateTime? ActionDate { get; set; }

    [MaxLength(500)]
    public string? ApproverRemarks { get; set; }

    public decimal? ApprovedAmount { get; set; }
    public DateTime SubmittedAt { get; set; } = DateTime.UtcNow;

    public Employee Employee { get; set; } = null!;
    public ExpenseCategory Category { get; set; } = null!;
}
