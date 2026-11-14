using Microsoft.EntityFrameworkCore;
using ZingHR.Api.Models;

namespace ZingHR.Api.Data;

public class AppDbContext : DbContext
{
    public AppDbContext(DbContextOptions<AppDbContext> options) : base(options) { }

    public DbSet<Company> Companies => Set<Company>();
    public DbSet<AppUser> Users => Set<AppUser>();
    public DbSet<Department> Departments => Set<Department>();
    public DbSet<Designation> Designations => Set<Designation>();
    public DbSet<Employee> Employees => Set<Employee>();
    public DbSet<Shift> Shifts => Set<Shift>();
    public DbSet<Attendance> Attendances => Set<Attendance>();
    public DbSet<LeaveType> LeaveTypes => Set<LeaveType>();
    public DbSet<LeaveBalance> LeaveBalances => Set<LeaveBalance>();
    public DbSet<LeaveRequest> LeaveRequests => Set<LeaveRequest>();
    public DbSet<SalaryStructure> SalaryStructures => Set<SalaryStructure>();
    public DbSet<Payslip> Payslips => Set<Payslip>();
    public DbSet<ExpenseCategory> ExpenseCategories => Set<ExpenseCategory>();
    public DbSet<Expense> Expenses => Set<Expense>();
    public DbSet<HelpTicket> HelpTickets => Set<HelpTicket>();
    public DbSet<TicketComment> TicketComments => Set<TicketComment>();
    public DbSet<Notification> Notifications => Set<Notification>();
    public DbSet<Announcement> Announcements => Set<Announcement>();
    public DbSet<Holiday> Holidays => Set<Holiday>();

    protected override void OnModelCreating(ModelBuilder mb)
    {
        mb.Entity<Employee>()
            .HasIndex(e => e.EmpCode).IsUnique();
        mb.Entity<Employee>()
            .HasOne(e => e.Department).WithMany(d => d.Employees).HasForeignKey(e => e.DepartmentId);
        mb.Entity<Employee>()
            .HasOne(e => e.Designation).WithMany().HasForeignKey(e => e.DesignationId);
        mb.Entity<Employee>()
            .HasOne(e => e.ReportingManager).WithMany().HasForeignKey(e => e.ReportingManagerId);
        mb.Entity<Employee>()
            .HasOne(e => e.Shift).WithMany(s => s.Employees).HasForeignKey(e => e.ShiftId);

        mb.Entity<AppUser>()
            .HasIndex(u => u.Username).IsUnique();
        mb.Entity<AppUser>()
            .HasOne(u => u.Employee).WithMany().HasForeignKey(u => u.EmployeeId);

        mb.Entity<Attendance>()
            .HasIndex(a => new { a.EmployeeId, a.Date }).IsUnique();

        mb.Entity<LeaveBalance>()
            .HasIndex(lb => new { lb.EmployeeId, lb.LeaveTypeId, lb.Year }).IsUnique();

        mb.Entity<Payslip>()
            .HasIndex(p => new { p.EmployeeId, p.Month, p.Year }).IsUnique();

        mb.Entity<HelpTicket>()
            .HasIndex(t => t.TicketNo).IsUnique();

        // Seed data
        SeedData(mb);
    }

    private static void SeedData(ModelBuilder mb)
    {
        mb.Entity<Company>().HasData(new Company
        {
            Id = 1,
            Name = "TechCorp India Pvt. Ltd.",
            Address = "Plot 42, Sector 18, Cyber City",
            City = "Gurugram",
            State = "Haryana",
            PinCode = "122015",
            Email = "hr@techcorp.in",
            Phone = "+91-124-4567890"
        });

        mb.Entity<Department>().HasData(
            new Department { Id = 1, Name = "Engineering", Code = "ENG" },
            new Department { Id = 2, Name = "Human Resources", Code = "HR" },
            new Department { Id = 3, Name = "Finance", Code = "FIN" },
            new Department { Id = 4, Name = "Marketing", Code = "MKT" },
            new Department { Id = 5, Name = "Operations", Code = "OPS" },
            new Department { Id = 6, Name = "Sales", Code = "SAL" }
        );

        mb.Entity<Designation>().HasData(
            new Designation { Id = 1, Title = "Software Engineer", Grade = "L1", Level = 1 },
            new Designation { Id = 2, Title = "Senior Software Engineer", Grade = "L2", Level = 2 },
            new Designation { Id = 3, Title = "Tech Lead", Grade = "L3", Level = 3 },
            new Designation { Id = 4, Title = "Engineering Manager", Grade = "L4", Level = 4 },
            new Designation { Id = 5, Title = "HR Executive", Grade = "L1", Level = 1 },
            new Designation { Id = 6, Title = "HR Manager", Grade = "L3", Level = 3 },
            new Designation { Id = 7, Title = "Accountant", Grade = "L1", Level = 1 },
            new Designation { Id = 8, Title = "Finance Manager", Grade = "L3", Level = 3 },
            new Designation { Id = 9, Title = "Marketing Executive", Grade = "L1", Level = 1 },
            new Designation { Id = 10, Title = "Director", Grade = "L5", Level = 5 }
        );

        mb.Entity<Shift>().HasData(
            new Shift { Id = 1, Name = "General Shift", Code = "GEN", StartTime = new TimeOnly(9, 0), EndTime = new TimeOnly(18, 0) },
            new Shift { Id = 2, Name = "Morning Shift", Code = "MOR", StartTime = new TimeOnly(6, 0), EndTime = new TimeOnly(14, 0) },
            new Shift { Id = 3, Name = "Night Shift", Code = "NGT", StartTime = new TimeOnly(22, 0), EndTime = new TimeOnly(6, 0), IsNightShift = true }
        );

        mb.Entity<LeaveType>().HasData(
            new LeaveType { Id = 1, Name = "Casual Leave", Code = "CL", DefaultDays = 12, Color = "#4CAF50" },
            new LeaveType { Id = 2, Name = "Sick Leave", Code = "SL", DefaultDays = 12, Color = "#F44336" },
            new LeaveType { Id = 3, Name = "Earned Leave", Code = "EL", DefaultDays = 15, IsCarryForward = true, MaxCarryForward = 30, IsEncashable = true, Color = "#2196F3" },
            new LeaveType { Id = 4, Name = "Maternity Leave", Code = "ML", DefaultDays = 182, IsPaid = true, AllowHalfDay = false, Color = "#E91E63" },
            new LeaveType { Id = 5, Name = "Paternity Leave", Code = "PL", DefaultDays = 15, IsPaid = true, AllowHalfDay = false, Color = "#9C27B0" },
            new LeaveType { Id = 6, Name = "Compensatory Off", Code = "CO", DefaultDays = 0, Color = "#FF9800" },
            new LeaveType { Id = 7, Name = "Loss of Pay", Code = "LOP", DefaultDays = 0, IsPaid = false, RequiresApproval = true, Color = "#607D8B" }
        );

        mb.Entity<ExpenseCategory>().HasData(
            new ExpenseCategory { Id = 1, Name = "Travel", MaxLimit = 50000 },
            new ExpenseCategory { Id = 2, Name = "Food & Meals", MaxLimit = 5000 },
            new ExpenseCategory { Id = 3, Name = "Accommodation", MaxLimit = 30000 },
            new ExpenseCategory { Id = 4, Name = "Office Supplies", MaxLimit = 10000 },
            new ExpenseCategory { Id = 5, Name = "Client Entertainment", MaxLimit = 15000 },
            new ExpenseCategory { Id = 6, Name = "Communication", MaxLimit = 3000 },
            new ExpenseCategory { Id = 7, Name = "Training & Certification", MaxLimit = 25000 },
            new ExpenseCategory { Id = 8, Name = "Medical", MaxLimit = 10000, RequiresReceipt = true },
            new ExpenseCategory { Id = 9, Name = "Other", RequiresReceipt = false }
        );

        // Seed sample employees
        mb.Entity<Employee>().HasData(
            new Employee { Id = 1, EmpCode = "EMP001", FullName = "Rajesh Kumar", FirstName = "Rajesh", LastName = "Kumar", Email = "rajesh@techcorp.in", Phone = "9876543210", Gender = "Male", DateOfJoining = new DateOnly(2020, 3, 15), DepartmentId = 1, DesignationId = 4, ShiftId = 1, EmploymentType = "full-time" },
            new Employee { Id = 2, EmpCode = "EMP002", FullName = "Priya Sharma", FirstName = "Priya", LastName = "Sharma", Email = "priya@techcorp.in", Phone = "9876543211", Gender = "Female", DateOfJoining = new DateOnly(2021, 6, 1), DepartmentId = 2, DesignationId = 6, ReportingManagerId = 1, ShiftId = 1, EmploymentType = "full-time" },
            new Employee { Id = 3, EmpCode = "EMP003", FullName = "Amit Patel", FirstName = "Amit", LastName = "Patel", Email = "amit@techcorp.in", Phone = "9876543212", Gender = "Male", DateOfJoining = new DateOnly(2022, 1, 10), DepartmentId = 1, DesignationId = 2, ReportingManagerId = 1, ShiftId = 1, EmploymentType = "full-time" },
            new Employee { Id = 4, EmpCode = "EMP004", FullName = "Sneha Reddy", FirstName = "Sneha", LastName = "Reddy", Email = "sneha@techcorp.in", Phone = "9876543213", Gender = "Female", DateOfJoining = new DateOnly(2023, 4, 20), DepartmentId = 3, DesignationId = 8, ReportingManagerId = 1, ShiftId = 1, EmploymentType = "full-time" },
            new Employee { Id = 5, EmpCode = "EMP005", FullName = "Vikram Singh", FirstName = "Vikram", LastName = "Singh", Email = "vikram@techcorp.in", Phone = "9876543214", Gender = "Male", DateOfJoining = new DateOnly(2023, 8, 5), DepartmentId = 1, DesignationId = 1, ReportingManagerId = 3, ShiftId = 1, EmploymentType = "full-time" }
        );

        // Seed users (password: admin123 for admin, emp123 for employees)
        mb.Entity<AppUser>().HasData(
            new AppUser { Id = 1, Username = "admin", PasswordHash = "$2a$11$rZnKqKbvMjBzRHNBpHDGnOJGdFPp3QCkXaRgfJEaH0GmXrNopZL2y", Role = "admin", FullName = "System Admin", Email = "admin@techcorp.in" },
            new AppUser { Id = 2, Username = "rajesh", PasswordHash = "$2a$11$Bf/LmPq1GXNW7vB0fH5w1eYfEHzq5Vc6L8jKm4rCE4qUlT0H1LXi", Role = "manager", EmployeeId = 1, FullName = "Rajesh Kumar", Email = "rajesh@techcorp.in" },
            new AppUser { Id = 3, Username = "priya", PasswordHash = "$2a$11$Bf/LmPq1GXNW7vB0fH5w1eYfEHzq5Vc6L8jKm4rCE4qUlT0H1LXi", Role = "hr", EmployeeId = 2, FullName = "Priya Sharma", Email = "priya@techcorp.in" },
            new AppUser { Id = 4, Username = "amit", PasswordHash = "$2a$11$Bf/LmPq1GXNW7vB0fH5w1eYfEHzq5Vc6L8jKm4rCE4qUlT0H1LXi", Role = "employee", EmployeeId = 3, FullName = "Amit Patel", Email = "amit@techcorp.in" },
            new AppUser { Id = 5, Username = "sneha", PasswordHash = "$2a$11$Bf/LmPq1GXNW7vB0fH5w1eYfEHzq5Vc6L8jKm4rCE4qUlT0H1LXi", Role = "employee", EmployeeId = 4, FullName = "Sneha Reddy", Email = "sneha@techcorp.in" },
            new AppUser { Id = 6, Username = "vikram", PasswordHash = "$2a$11$Bf/LmPq1GXNW7vB0fH5w1eYfEHzq5Vc6L8jKm4rCE4qUlT0H1LXi", Role = "employee", EmployeeId = 5, FullName = "Vikram Singh", Email = "vikram@techcorp.in" }
        );

        // Seed leave balances for current year
        var year = DateTime.UtcNow.Year;
        int balId = 1;
        for (int empId = 1; empId <= 5; empId++)
        {
            mb.Entity<LeaveBalance>().HasData(
                new LeaveBalance { Id = balId++, EmployeeId = empId, LeaveTypeId = 1, Year = year, Entitled = 12, Used = 0, Balance = 12 },
                new LeaveBalance { Id = balId++, EmployeeId = empId, LeaveTypeId = 2, Year = year, Entitled = 12, Used = 0, Balance = 12 },
                new LeaveBalance { Id = balId++, EmployeeId = empId, LeaveTypeId = 3, Year = year, Entitled = 15, Used = 0, Balance = 15 }
            );
        }

        // Seed holidays
        mb.Entity<Holiday>().HasData(
            new Holiday { Id = 1, Name = "Republic Day", Date = new DateOnly(year, 1, 26), Year = year },
            new Holiday { Id = 2, Name = "Holi", Date = new DateOnly(year, 3, 14), Year = year },
            new Holiday { Id = 3, Name = "Good Friday", Date = new DateOnly(year, 3, 29), Year = year, Type = "restricted" },
            new Holiday { Id = 4, Name = "Independence Day", Date = new DateOnly(year, 8, 15), Year = year },
            new Holiday { Id = 5, Name = "Gandhi Jayanti", Date = new DateOnly(year, 10, 2), Year = year },
            new Holiday { Id = 6, Name = "Diwali", Date = new DateOnly(year, 11, 1), Year = year },
            new Holiday { Id = 7, Name = "Christmas", Date = new DateOnly(year, 12, 25), Year = year }
        );

        // Seed announcements
        mb.Entity<Announcement>().HasData(
            new Announcement { Id = 1, Title = "Welcome to ZingHR Portal", Content = "We are excited to launch our new HR management system. Please complete your profile and explore the features.", Category = "general", Priority = "high", IsPublished = true, CreatedBy = 1 },
            new Announcement { Id = 2, Title = "Updated Leave Policy 2026", Content = "Please review the updated leave policy effective from January 2026. Key changes include increased earned leave quota and new work-from-home leave category.", Category = "policy", Priority = "normal", IsPublished = true, CreatedBy = 1 }
        );
    }
}
