using System.ComponentModel.DataAnnotations;

namespace ZingHR.Api.Models;

public class Department
{
    public int Id { get; set; }

    [Required, MaxLength(200)]
    public string Name { get; set; } = "";

    [MaxLength(20)]
    public string Code { get; set; } = "";

    public int? HeadEmployeeId { get; set; }
    public bool IsActive { get; set; } = true;

    public Employee? Head { get; set; }
    public ICollection<Employee> Employees { get; set; } = new List<Employee>();
}

public class Designation
{
    public int Id { get; set; }

    [Required, MaxLength(200)]
    public string Title { get; set; } = "";

    [MaxLength(10)]
    public string Grade { get; set; } = "";

    public int Level { get; set; }
    public bool IsActive { get; set; } = true;
}
