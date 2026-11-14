using System.ComponentModel.DataAnnotations;

namespace ZingHR.Api.Models;

public class Company
{
    public int Id { get; set; }

    [Required, MaxLength(300)]
    public string Name { get; set; } = "";

    [MaxLength(500)]
    public string Address { get; set; } = "";

    [MaxLength(100)]
    public string City { get; set; } = "";

    [MaxLength(100)]
    public string State { get; set; } = "";

    [MaxLength(20)]
    public string PinCode { get; set; } = "";

    [MaxLength(200)]
    public string Email { get; set; } = "";

    [MaxLength(20)]
    public string Phone { get; set; } = "";

    [MaxLength(500)]
    public string LogoUrl { get; set; } = "";

    [MaxLength(50)]
    public string Timezone { get; set; } = "Asia/Kolkata";

    [MaxLength(10)]
    public string Currency { get; set; } = "INR";

    public string WorkingDays { get; set; } = "Mon,Tue,Wed,Thu,Fri";
    public TimeOnly ShiftStart { get; set; } = new(9, 0);
    public TimeOnly ShiftEnd { get; set; } = new(18, 0);
}
