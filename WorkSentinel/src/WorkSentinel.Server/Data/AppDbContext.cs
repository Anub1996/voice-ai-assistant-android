using Microsoft.EntityFrameworkCore;
using WorkSentinel.Server.Models;

namespace WorkSentinel.Server.Data;

public class AppDbContext : DbContext
{
    public AppDbContext(DbContextOptions<AppDbContext> options) : base(options) { }

    public DbSet<AdminUser> AdminUsers => Set<AdminUser>();
    public DbSet<Employee> Employees => Set<Employee>();
    public DbSet<ActivityLog> ActivityLogs => Set<ActivityLog>();
    public DbSet<AppUsage> AppUsages => Set<AppUsage>();
    public DbSet<WebVisit> WebVisits => Set<WebVisit>();
    public DbSet<ScreenCapture> ScreenCaptures => Set<ScreenCapture>();
    public DbSet<EmailLog> EmailLogs => Set<EmailLog>();
    public DbSet<FileActivity> FileActivities => Set<FileActivity>();
    public DbSet<DlpPolicy> DlpPolicies => Set<DlpPolicy>();
    public DbSet<Alert> Alerts => Set<Alert>();
    public DbSet<ProductivityScore> ProductivityScores => Set<ProductivityScore>();
    public DbSet<AppCategory> AppCategories => Set<AppCategory>();
    public DbSet<WebCategory> WebCategories => Set<WebCategory>();

    protected override void OnModelCreating(ModelBuilder b)
    {
        b.Entity<Employee>().HasIndex(e => e.EmployeeId).IsUnique();
        b.Entity<AdminUser>().HasIndex(u => u.Username).IsUnique();

        b.Entity<ActivityLog>().HasIndex(a => new { a.EmployeeId, a.StartTime });
        b.Entity<AppUsage>().HasIndex(a => new { a.EmployeeId, a.StartTime });
        b.Entity<WebVisit>().HasIndex(w => new { w.EmployeeId, w.VisitTime });
        b.Entity<EmailLog>().HasIndex(e => new { e.EmployeeId, e.Timestamp });
        b.Entity<FileActivity>().HasIndex(f => new { f.EmployeeId, f.Timestamp });
        b.Entity<Alert>().HasIndex(a => new { a.EmployeeId, a.Timestamp });
        b.Entity<Alert>().HasIndex(a => a.Acknowledged);
        b.Entity<ProductivityScore>().HasIndex(p => new { p.EmployeeId, p.Date }).IsUnique();

        b.Entity<AppCategory>().HasIndex(c => c.AppNamePattern);
        b.Entity<WebCategory>().HasIndex(c => c.DomainPattern);

        // Seed default admin
        b.Entity<AdminUser>().HasData(new AdminUser
        {
            Id = 1,
            Username = "admin",
            PasswordHash = BCrypt.Net.BCrypt.HashPassword("admin123"),
            FullName = "System Administrator",
            Email = "admin@worksentinel.local",
            Role = "admin"
        });

        // Seed app categories
        b.Entity<AppCategory>().HasData(
            new AppCategory { Id = 1, AppNamePattern = "Visual Studio", Category = "productive", DisplayName = "Visual Studio" },
            new AppCategory { Id = 2, AppNamePattern = "Code", Category = "productive", DisplayName = "VS Code" },
            new AppCategory { Id = 3, AppNamePattern = "Excel", Category = "productive", DisplayName = "Microsoft Excel" },
            new AppCategory { Id = 4, AppNamePattern = "Word", Category = "productive", DisplayName = "Microsoft Word" },
            new AppCategory { Id = 5, AppNamePattern = "PowerPoint", Category = "productive", DisplayName = "PowerPoint" },
            new AppCategory { Id = 6, AppNamePattern = "Outlook", Category = "productive", DisplayName = "Outlook" },
            new AppCategory { Id = 7, AppNamePattern = "Teams", Category = "productive", DisplayName = "Microsoft Teams" },
            new AppCategory { Id = 8, AppNamePattern = "Slack", Category = "productive", DisplayName = "Slack" },
            new AppCategory { Id = 9, AppNamePattern = "Spotify", Category = "unproductive", DisplayName = "Spotify" },
            new AppCategory { Id = 10, AppNamePattern = "Netflix", Category = "unproductive", DisplayName = "Netflix" },
            new AppCategory { Id = 11, AppNamePattern = "Steam", Category = "unproductive", DisplayName = "Steam" },
            new AppCategory { Id = 12, AppNamePattern = "Discord", Category = "unproductive", DisplayName = "Discord" }
        );

        // Seed web categories
        b.Entity<WebCategory>().HasData(
            new WebCategory { Id = 1, DomainPattern = "github.com", Category = "productive", DisplayName = "GitHub" },
            new WebCategory { Id = 2, DomainPattern = "stackoverflow.com", Category = "productive", DisplayName = "Stack Overflow" },
            new WebCategory { Id = 3, DomainPattern = "docs.microsoft.com", Category = "productive", DisplayName = "Microsoft Docs" },
            new WebCategory { Id = 4, DomainPattern = "learn.microsoft.com", Category = "productive", DisplayName = "Microsoft Learn" },
            new WebCategory { Id = 5, DomainPattern = "jira", Category = "productive", DisplayName = "Jira" },
            new WebCategory { Id = 6, DomainPattern = "confluence", Category = "productive", DisplayName = "Confluence" },
            new WebCategory { Id = 7, DomainPattern = "facebook.com", Category = "unproductive", DisplayName = "Facebook" },
            new WebCategory { Id = 8, DomainPattern = "instagram.com", Category = "unproductive", DisplayName = "Instagram" },
            new WebCategory { Id = 9, DomainPattern = "twitter.com", Category = "unproductive", DisplayName = "Twitter/X" },
            new WebCategory { Id = 10, DomainPattern = "youtube.com", Category = "unproductive", DisplayName = "YouTube" },
            new WebCategory { Id = 11, DomainPattern = "reddit.com", Category = "unproductive", DisplayName = "Reddit" },
            new WebCategory { Id = 12, DomainPattern = "tiktok.com", Category = "unproductive", DisplayName = "TikTok" }
        );
    }
}
