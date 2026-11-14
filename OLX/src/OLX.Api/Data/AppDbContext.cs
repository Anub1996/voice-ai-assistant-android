using Microsoft.EntityFrameworkCore;
using OLX.Api.Models;

namespace OLX.Api.Data;

public class AppDbContext : DbContext
{
    public AppDbContext(DbContextOptions<AppDbContext> options) : base(options) { }

    public DbSet<User> Users => Set<User>();
    public DbSet<Category> Categories => Set<Category>();
    public DbSet<Listing> Listings => Set<Listing>();
    public DbSet<ListingImage> ListingImages => Set<ListingImage>();
    public DbSet<Conversation> Conversations => Set<Conversation>();
    public DbSet<Message> Messages => Set<Message>();
    public DbSet<Favorite> Favorites => Set<Favorite>();
    public DbSet<SavedSearch> SavedSearches => Set<SavedSearch>();
    public DbSet<Notification> Notifications => Set<Notification>();
    public DbSet<Report> Reports => Set<Report>();

    protected override void OnModelCreating(ModelBuilder mb)
    {
        mb.Entity<User>().HasIndex(u => u.Username).IsUnique();
        mb.Entity<User>().HasIndex(u => u.Email).IsUnique();
        mb.Entity<Listing>().HasIndex(l => l.Slug).IsUnique();
        mb.Entity<Category>().HasIndex(c => c.Slug).IsUnique();
        mb.Entity<Favorite>().HasIndex(f => new { f.UserId, f.ListingId }).IsUnique();
        mb.Entity<Conversation>().HasIndex(c => new { c.ListingId, c.BuyerId }).IsUnique();

        mb.Entity<Conversation>()
            .HasOne(c => c.Buyer).WithMany().HasForeignKey(c => c.BuyerId).OnDelete(DeleteBehavior.Restrict);
        mb.Entity<Conversation>()
            .HasOne(c => c.SellerUser).WithMany().HasForeignKey(c => c.SellerId).OnDelete(DeleteBehavior.Restrict);
        mb.Entity<Message>()
            .HasOne(m => m.Sender).WithMany().HasForeignKey(m => m.SenderId).OnDelete(DeleteBehavior.Restrict);
        mb.Entity<Report>()
            .HasOne(r => r.Reporter).WithMany().HasForeignKey(r => r.ReporterId).OnDelete(DeleteBehavior.Restrict);

        SeedData(mb);
    }

    private static void SeedData(ModelBuilder mb)
    {
        // Categories
        mb.Entity<Category>().HasData(
            new Category { Id = 1, Name = "Cars", Slug = "cars", Icon = "🚗", SortOrder = 1 },
            new Category { Id = 2, Name = "Motorcycles", Slug = "motorcycles", Icon = "🏍️", SortOrder = 2 },
            new Category { Id = 3, Name = "Mobile Phones", Slug = "mobile-phones", Icon = "📱", SortOrder = 3 },
            new Category { Id = 4, Name = "Electronics", Slug = "electronics", Icon = "💻", SortOrder = 4 },
            new Category { Id = 5, Name = "Furniture", Slug = "furniture", Icon = "🛋️", SortOrder = 5 },
            new Category { Id = 6, Name = "Fashion", Slug = "fashion", Icon = "👕", SortOrder = 6 },
            new Category { Id = 7, Name = "Real Estate", Slug = "real-estate", Icon = "🏠", SortOrder = 7 },
            new Category { Id = 8, Name = "Jobs", Slug = "jobs", Icon = "💼", SortOrder = 8 },
            new Category { Id = 9, Name = "Services", Slug = "services", Icon = "🔧", SortOrder = 9 },
            new Category { Id = 10, Name = "Books & Sports", Slug = "books-sports", Icon = "📚", SortOrder = 10 },
            // Sub-categories
            new Category { Id = 11, Name = "Hatchback", Slug = "hatchback", Icon = "🚗", ParentId = 1, SortOrder = 1 },
            new Category { Id = 12, Name = "Sedan", Slug = "sedan", Icon = "🚗", ParentId = 1, SortOrder = 2 },
            new Category { Id = 13, Name = "SUV", Slug = "suv", Icon = "🚗", ParentId = 1, SortOrder = 3 },
            new Category { Id = 14, Name = "Smartphones", Slug = "smartphones", Icon = "📱", ParentId = 3, SortOrder = 1 },
            new Category { Id = 15, Name = "Tablets", Slug = "tablets", Icon = "📱", ParentId = 3, SortOrder = 2 },
            new Category { Id = 16, Name = "Laptops", Slug = "laptops", Icon = "💻", ParentId = 4, SortOrder = 1 },
            new Category { Id = 17, Name = "TVs", Slug = "tvs", Icon = "📺", ParentId = 4, SortOrder = 2 },
            new Category { Id = 18, Name = "Cameras", Slug = "cameras", Icon = "📷", ParentId = 4, SortOrder = 3 },
            new Category { Id = 19, Name = "Apartments", Slug = "apartments", Icon = "🏠", ParentId = 7, SortOrder = 1 },
            new Category { Id = 20, Name = "Houses", Slug = "houses", Icon = "🏡", ParentId = 7, SortOrder = 2 }
        );

        // Users
        var pw = BCrypt.Net.BCrypt.HashPassword("demo123");
        mb.Entity<User>().HasData(
            new User { Id = 1, Username = "admin", Email = "admin@olxclone.com", Phone = "9999999999", PasswordHash = pw, FullName = "Admin User", City = "Mumbai", State = "Maharashtra", IsVerified = true, JoinedAt = DateTime.Parse("2024-01-01") },
            new User { Id = 2, Username = "rahul", Email = "rahul@gmail.com", Phone = "9876543210", PasswordHash = pw, FullName = "Rahul Sharma", City = "Delhi", State = "Delhi", IsVerified = true, JoinedAt = DateTime.Parse("2024-03-15") },
            new User { Id = 3, Username = "priya", Email = "priya@gmail.com", Phone = "9876543211", PasswordHash = pw, FullName = "Priya Patel", City = "Bangalore", State = "Karnataka", IsVerified = true, JoinedAt = DateTime.Parse("2024-06-01") },
            new User { Id = 4, Username = "amit", Email = "amit@gmail.com", Phone = "9876543212", PasswordHash = pw, FullName = "Amit Kumar", City = "Hyderabad", State = "Telangana", JoinedAt = DateTime.Parse("2024-08-20") },
            new User { Id = 5, Username = "neha", Email = "neha@gmail.com", Phone = "9876543213", PasswordHash = pw, FullName = "Neha Singh", City = "Pune", State = "Maharashtra", IsVerified = true, JoinedAt = DateTime.Parse("2024-09-10") }
        );

        // Listings
        mb.Entity<Listing>().HasData(
            new Listing { Id = 1, Title = "Maruti Swift VXi 2020", Slug = "maruti-swift-vxi-2020", Description = "Well maintained, single owner, 25000 km driven. All service records available. Insurance valid till 2025.", Price = 550000, IsNegotiable = true, Condition = "used", CategoryId = 11, SellerId = 2, City = "Delhi", State = "Delhi", Latitude = 28.6139, Longitude = 77.2090, ViewCount = 142, CreatedAt = DateTime.Parse("2024-10-01"), ExpiresAt = DateTime.Parse("2025-01-01") },
            new Listing { Id = 2, Title = "iPhone 14 Pro Max 256GB", Slug = "iphone-14-pro-max-256gb", Description = "Space Black, 98% battery health, all accessories included with box. No scratches.", Price = 85000, IsNegotiable = true, Condition = "like_new", CategoryId = 14, SellerId = 3, City = "Bangalore", State = "Karnataka", ViewCount = 89, CreatedAt = DateTime.Parse("2024-10-15"), ExpiresAt = DateTime.Parse("2025-01-15") },
            new Listing { Id = 3, Title = "MacBook Air M2 2023", Slug = "macbook-air-m2-2023", Description = "8GB RAM, 256GB SSD. Used for 6 months only. Perfect condition with original charger.", Price = 78000, IsNegotiable = false, Condition = "like_new", CategoryId = 16, SellerId = 2, City = "Delhi", State = "Delhi", ViewCount = 256, IsFeatured = true, CreatedAt = DateTime.Parse("2024-11-01"), ExpiresAt = DateTime.Parse("2025-02-01") },
            new Listing { Id = 4, Title = "Royal Enfield Classic 350", Slug = "royal-enfield-classic-350", Description = "2022 model, 8000 km, Stealth Black. Aftermarket exhaust and crash guard.", Price = 165000, IsNegotiable = true, Condition = "used", CategoryId = 2, SellerId = 4, City = "Hyderabad", State = "Telangana", ViewCount = 67, CreatedAt = DateTime.Parse("2024-11-10") },
            new Listing { Id = 5, Title = "L-shaped Sofa Set", Slug = "l-shaped-sofa-set", Description = "Premium fabric sofa, 3 years old, minor wear. Moving out sale.", Price = 15000, IsNegotiable = true, Condition = "used", CategoryId = 5, SellerId = 5, City = "Pune", State = "Maharashtra", ViewCount = 34, CreatedAt = DateTime.Parse("2024-11-20") },
            new Listing { Id = 6, Title = "Samsung 55\" 4K Smart TV", Slug = "samsung-55-4k-smart-tv", Description = "Crystal UHD, bought in 2023. Works perfectly. Remote and wall mount included.", Price = 28000, IsNegotiable = true, Condition = "used", CategoryId = 17, SellerId = 3, City = "Bangalore", State = "Karnataka", ViewCount = 112, CreatedAt = DateTime.Parse("2024-12-01") },
            new Listing { Id = 7, Title = "2BHK Flat for Rent Koramangala", Slug = "2bhk-flat-rent-koramangala", Description = "Fully furnished, 2nd floor, gated community. Close to metro. Rent 25k + maintenance.", Price = 25000, Condition = "new", CategoryId = 19, SellerId = 3, City = "Bangalore", State = "Karnataka", ViewCount = 455, IsFeatured = true, CreatedAt = DateTime.Parse("2024-12-05") },
            new Listing { Id = 8, Title = "Nikon D5600 DSLR Camera", Slug = "nikon-d5600-dslr-camera", Description = "With 18-55mm and 70-300mm lens. Bag, extra battery, 64GB card included.", Price = 42000, IsNegotiable = true, Condition = "used", CategoryId = 18, SellerId = 4, City = "Hyderabad", State = "Telangana", ViewCount = 78, CreatedAt = DateTime.Parse("2024-12-10") }
        );

        // Listing Images
        mb.Entity<ListingImage>().HasData(
            new ListingImage { Id = 1, ListingId = 1, Url = "/images/car1.jpg", SortOrder = 0 },
            new ListingImage { Id = 2, ListingId = 2, Url = "/images/phone1.jpg", SortOrder = 0 },
            new ListingImage { Id = 3, ListingId = 3, Url = "/images/laptop1.jpg", SortOrder = 0 },
            new ListingImage { Id = 4, ListingId = 4, Url = "/images/bike1.jpg", SortOrder = 0 },
            new ListingImage { Id = 5, ListingId = 5, Url = "/images/sofa1.jpg", SortOrder = 0 },
            new ListingImage { Id = 6, ListingId = 6, Url = "/images/tv1.jpg", SortOrder = 0 },
            new ListingImage { Id = 7, ListingId = 7, Url = "/images/flat1.jpg", SortOrder = 0 },
            new ListingImage { Id = 8, ListingId = 8, Url = "/images/camera1.jpg", SortOrder = 0 }
        );

        // Favorites
        mb.Entity<Favorite>().HasData(
            new Favorite { Id = 1, UserId = 2, ListingId = 2, CreatedAt = DateTime.Parse("2024-11-01") },
            new Favorite { Id = 2, UserId = 3, ListingId = 1, CreatedAt = DateTime.Parse("2024-11-05") },
            new Favorite { Id = 3, UserId = 5, ListingId = 3, CreatedAt = DateTime.Parse("2024-11-15") }
        );

        // Conversations
        mb.Entity<Conversation>().HasData(
            new Conversation { Id = 1, ListingId = 1, BuyerId = 3, SellerId = 2, CreatedAt = DateTime.Parse("2024-11-05"), LastMessageAt = DateTime.Parse("2024-11-05") },
            new Conversation { Id = 2, ListingId = 3, BuyerId = 5, SellerId = 2, CreatedAt = DateTime.Parse("2024-11-20"), LastMessageAt = DateTime.Parse("2024-11-20") }
        );

        // Messages
        mb.Entity<Message>().HasData(
            new Message { Id = 1, ConversationId = 1, SenderId = 3, Text = "Hi, is this car still available?", SentAt = DateTime.Parse("2024-11-05T10:00:00"), IsRead = true },
            new Message { Id = 2, ConversationId = 1, SenderId = 2, Text = "Yes, it is. Would you like to see it?", SentAt = DateTime.Parse("2024-11-05T10:05:00"), IsRead = true },
            new Message { Id = 3, ConversationId = 2, SenderId = 5, Text = "Can you do 70k for the MacBook?", SentAt = DateTime.Parse("2024-11-20T14:00:00"), IsRead = false }
        );
    }
}
