using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using OLX.Api.Data;
using OLX.Api.Models;
using OLX.Api.Services;

namespace OLX.Api.Controllers;

[ApiController, Route("api/[controller]")]
public class ListingsController : ControllerBase
{
    private readonly AppDbContext _db;
    private readonly NotificationService _notif;

    public ListingsController(AppDbContext db, NotificationService notif) { _db = db; _notif = notif; }

    private int? CurrentUserId => int.TryParse(User.FindFirstValue(ClaimTypes.NameIdentifier), out var id) ? id : null;

    [HttpGet]
    public async Task<IActionResult> GetAll([FromQuery] ListingSearchRequest req)
    {
        var q = _db.Listings.Include(l => l.Category).Include(l => l.Seller).Include(l => l.Images)
            .Where(l => l.Status == "active");

        if (!string.IsNullOrWhiteSpace(req.Query))
            q = q.Where(l => l.Title.Contains(req.Query) || l.Description.Contains(req.Query));
        if (req.CategoryId.HasValue)
            q = q.Where(l => l.CategoryId == req.CategoryId || l.Category.ParentId == req.CategoryId);
        if (!string.IsNullOrWhiteSpace(req.City)) q = q.Where(l => l.City == req.City);
        if (!string.IsNullOrWhiteSpace(req.State)) q = q.Where(l => l.State == req.State);
        if (req.MinPrice.HasValue) q = q.Where(l => l.Price >= req.MinPrice);
        if (req.MaxPrice.HasValue) q = q.Where(l => l.Price <= req.MaxPrice);
        if (!string.IsNullOrWhiteSpace(req.Condition)) q = q.Where(l => l.Condition == req.Condition);

        q = req.SortBy switch
        {
            "price_asc" => q.OrderBy(l => l.Price),
            "price_desc" => q.OrderByDescending(l => l.Price),
            "popular" => q.OrderByDescending(l => l.ViewCount),
            _ => q.OrderByDescending(l => l.IsFeatured).ThenByDescending(l => l.CreatedAt)
        };

        var total = await q.CountAsync();
        var items = await q.Skip((req.Page - 1) * req.PageSize).Take(req.PageSize).ToListAsync();
        var uid = CurrentUserId;
        var favIds = uid.HasValue
            ? (await _db.Favorites.Where(f => f.UserId == uid).Select(f => f.ListingId).ToListAsync()).ToHashSet()
            : new HashSet<long>();

        var dtos = items.Select(l => new ListingListDto(l.Id, l.Title, l.Slug, l.Price, l.Currency, l.IsNegotiable,
            l.Condition, l.City, l.State, l.Status, l.IsFeatured, l.ViewCount,
            l.Images.OrderBy(i => i.SortOrder).FirstOrDefault()?.Url, l.Category.Name,
            l.Seller.FullName, l.SellerId, l.CreatedAt, favIds.Contains(l.Id))).ToList();

        return Ok(new PagedResult<ListingListDto>(dtos, total, req.Page, req.PageSize, (int)Math.Ceiling(total / (double)req.PageSize)));
    }

    [HttpGet("featured")]
    public async Task<IActionResult> GetFeatured()
    {
        var items = await _db.Listings.Include(l => l.Category).Include(l => l.Seller).Include(l => l.Images)
            .Where(l => l.Status == "active" && l.IsFeatured).OrderByDescending(l => l.CreatedAt).Take(10).ToListAsync();

        return Ok(items.Select(l => new ListingListDto(l.Id, l.Title, l.Slug, l.Price, l.Currency, l.IsNegotiable,
            l.Condition, l.City, l.State, l.Status, l.IsFeatured, l.ViewCount,
            l.Images.OrderBy(i => i.SortOrder).FirstOrDefault()?.Url, l.Category.Name,
            l.Seller.FullName, l.SellerId, l.CreatedAt, false)));
    }

    [HttpGet("{id}")]
    public async Task<IActionResult> GetById(long id)
    {
        var l = await _db.Listings.Include(l => l.Category).ThenInclude(c => c.Parent)
            .Include(l => l.Seller).Include(l => l.Images).FirstOrDefaultAsync(l => l.Id == id);
        if (l == null) return NotFound();

        l.ViewCount++;
        await _db.SaveChangesAsync();

        var uid = CurrentUserId;
        var isFav = uid.HasValue && await _db.Favorites.AnyAsync(f => f.UserId == uid && f.ListingId == id);
        var favCount = await _db.Favorites.CountAsync(f => f.ListingId == id);

        var cat = new CategoryDto(l.Category.Id, l.Category.Name, l.Category.Slug, l.Category.Icon, l.Category.ParentId, null);
        var seller = new SellerDto(l.Seller.Id, l.Seller.Username, l.Seller.FullName, l.Seller.ProfilePhotoUrl,
            l.Seller.City, l.Seller.JoinedAt, l.Seller.IsVerified);

        return Ok(new ListingDetailDto(l.Id, l.Title, l.Slug, l.Description, l.Price, l.Currency, l.IsNegotiable,
            l.Condition, cat, seller, l.Latitude, l.Longitude, l.Address, l.City, l.State, l.Status, l.IsFeatured,
            l.ViewCount, l.Images.OrderBy(i => i.SortOrder).Select(i => i.Url).ToList(),
            l.CreatedAt, l.UpdatedAt, l.ExpiresAt, isFav, favCount));
    }

    [Authorize, HttpGet("my")]
    public async Task<IActionResult> GetMyListings()
    {
        var uid = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);
        var items = await _db.Listings.Include(l => l.Category).Include(l => l.Seller).Include(l => l.Images)
            .Where(l => l.SellerId == uid).OrderByDescending(l => l.CreatedAt).ToListAsync();

        return Ok(items.Select(l => new ListingListDto(l.Id, l.Title, l.Slug, l.Price, l.Currency, l.IsNegotiable,
            l.Condition, l.City, l.State, l.Status, l.IsFeatured, l.ViewCount,
            l.Images.OrderBy(i => i.SortOrder).FirstOrDefault()?.Url, l.Category.Name,
            l.Seller.FullName, l.SellerId, l.CreatedAt, false)));
    }

    [Authorize, HttpPost]
    public async Task<IActionResult> Create(CreateListingRequest req)
    {
        var uid = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);
        var slug = req.Title.ToLower().Replace(" ", "-").Replace("\"", "") + "-" + DateTimeOffset.UtcNow.ToUnixTimeSeconds();

        var listing = new Listing
        {
            Title = req.Title, Slug = slug, Description = req.Description, Price = req.Price,
            Currency = req.Currency ?? "INR", IsNegotiable = req.IsNegotiable, Condition = req.Condition,
            CategoryId = req.CategoryId, SellerId = uid, Latitude = req.Latitude, Longitude = req.Longitude,
            Address = req.Address, City = req.City, State = req.State, ExpiresAt = DateTime.UtcNow.AddDays(30)
        };
        _db.Listings.Add(listing);
        await _db.SaveChangesAsync();

        if (req.ImageUrls?.Any() == true)
        {
            for (int i = 0; i < req.ImageUrls.Count; i++)
                _db.ListingImages.Add(new ListingImage { ListingId = listing.Id, Url = req.ImageUrls[i], SortOrder = i });
            await _db.SaveChangesAsync();
        }

        // Notify saved-search users
        var matches = await _db.SavedSearches.Where(s => s.NotifyOnNew &&
            (s.CategoryId == null || s.CategoryId == req.CategoryId) &&
            (s.City == null || s.City == req.City) &&
            (s.MinPrice == null || req.Price >= s.MinPrice) &&
            (s.MaxPrice == null || req.Price <= s.MaxPrice)).ToListAsync();
        foreach (var s in matches)
            await _notif.CreateAsync(s.UserId, "new_listing", "New listing matches your search",
                req.Title, listing.Id);

        return CreatedAtAction(nameof(GetById), new { id = listing.Id }, new IdResponse("Listing created", listing.Id));
    }

    [Authorize, HttpPut("{id}")]
    public async Task<IActionResult> Update(long id, UpdateListingRequest req)
    {
        var uid = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);
        var listing = await _db.Listings.FirstOrDefaultAsync(l => l.Id == id && l.SellerId == uid);
        if (listing == null) return NotFound();

        if (req.Title != null) { listing.Title = req.Title; listing.Slug = req.Title.ToLower().Replace(" ", "-") + "-" + listing.Id; }
        if (req.Description != null) listing.Description = req.Description;
        if (req.Price.HasValue) listing.Price = req.Price.Value;
        if (req.IsNegotiable.HasValue) listing.IsNegotiable = req.IsNegotiable.Value;
        if (req.Condition != null) listing.Condition = req.Condition;
        if (req.CategoryId.HasValue) listing.CategoryId = req.CategoryId.Value;
        if (req.Address != null) listing.Address = req.Address;
        if (req.City != null) listing.City = req.City;
        if (req.State != null) listing.State = req.State;
        listing.UpdatedAt = DateTime.UtcNow;
        await _db.SaveChangesAsync();

        return Ok(new MessageResponse("Listing updated"));
    }

    [Authorize, HttpPut("{id}/status")]
    public async Task<IActionResult> UpdateStatus(long id, [FromBody] string status)
    {
        var uid = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);
        var listing = await _db.Listings.FirstOrDefaultAsync(l => l.Id == id && l.SellerId == uid);
        if (listing == null) return NotFound();

        listing.Status = status;
        listing.UpdatedAt = DateTime.UtcNow;
        await _db.SaveChangesAsync();

        if (status == "sold")
        {
            var favUsers = await _db.Favorites.Where(f => f.ListingId == id).Select(f => f.UserId).ToListAsync();
            foreach (var fu in favUsers)
                await _notif.CreateAsync(fu, "favorite_sold", "Item sold", $"{listing.Title} has been marked as sold", listing.Id);
        }

        return Ok(new MessageResponse("Status updated"));
    }

    [Authorize, HttpDelete("{id}")]
    public async Task<IActionResult> Delete(long id)
    {
        var uid = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);
        var listing = await _db.Listings.FirstOrDefaultAsync(l => l.Id == id && l.SellerId == uid);
        if (listing == null) return NotFound();

        listing.Status = "removed";
        listing.UpdatedAt = DateTime.UtcNow;
        await _db.SaveChangesAsync();
        return Ok(new MessageResponse("Listing removed"));
    }
}
