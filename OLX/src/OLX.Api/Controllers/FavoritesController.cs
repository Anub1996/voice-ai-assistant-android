using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using OLX.Api.Data;
using OLX.Api.Models;

namespace OLX.Api.Controllers;

[ApiController, Route("api/[controller]"), Authorize]
public class FavoritesController : ControllerBase
{
    private readonly AppDbContext _db;

    public FavoritesController(AppDbContext db) => _db = db;

    private int Uid => int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);

    [HttpGet]
    public async Task<IActionResult> GetAll()
    {
        var items = await _db.Favorites.Include(f => f.Listing).ThenInclude(l => l.Images)
            .Where(f => f.UserId == Uid).OrderByDescending(f => f.CreatedAt).ToListAsync();

        return Ok(items.Select(f => new FavoriteDto(f.ListingId, f.Listing.Title, f.Listing.Price,
            f.Listing.Currency, f.Listing.Images.OrderBy(i => i.SortOrder).FirstOrDefault()?.Url,
            f.Listing.City, f.Listing.Status, f.CreatedAt)));
    }

    [HttpPost("{listingId}")]
    public async Task<IActionResult> Add(long listingId)
    {
        var uid = Uid;
        if (await _db.Favorites.AnyAsync(f => f.UserId == uid && f.ListingId == listingId))
            return Conflict(new MessageResponse("Already in favorites"));
        if (!await _db.Listings.AnyAsync(l => l.Id == listingId))
            return NotFound();

        _db.Favorites.Add(new Favorite { UserId = uid, ListingId = listingId });
        await _db.SaveChangesAsync();
        return Ok(new MessageResponse("Added to favorites"));
    }

    [HttpDelete("{listingId}")]
    public async Task<IActionResult> Remove(long listingId)
    {
        var fav = await _db.Favorites.FirstOrDefaultAsync(f => f.UserId == Uid && f.ListingId == listingId);
        if (fav == null) return NotFound();
        _db.Favorites.Remove(fav);
        await _db.SaveChangesAsync();
        return Ok(new MessageResponse("Removed from favorites"));
    }

    [HttpGet("saved-searches")]
    public async Task<IActionResult> GetSavedSearches()
    {
        var items = await _db.SavedSearches.Include(s => s.User)
            .Where(s => s.UserId == Uid).OrderByDescending(s => s.CreatedAt).ToListAsync();

        var catIds = items.Where(s => s.CategoryId.HasValue).Select(s => s.CategoryId!.Value).Distinct().ToList();
        var catNames = await _db.Categories.Where(c => catIds.Contains(c.Id)).ToDictionaryAsync(c => c.Id, c => c.Name);

        return Ok(items.Select(s => new SavedSearchDto(s.Id, s.Query, s.CategoryId,
            s.CategoryId.HasValue ? catNames.GetValueOrDefault(s.CategoryId.Value) : null,
            s.City, s.MinPrice, s.MaxPrice, s.NotifyOnNew, s.CreatedAt)));
    }

    [HttpPost("saved-searches")]
    public async Task<IActionResult> SaveSearch(SaveSearchRequest req)
    {
        var ss = new SavedSearch
        {
            UserId = Uid, Query = req.Query, CategoryId = req.CategoryId,
            City = req.City, MinPrice = req.MinPrice, MaxPrice = req.MaxPrice, NotifyOnNew = req.NotifyOnNew
        };
        _db.SavedSearches.Add(ss);
        await _db.SaveChangesAsync();
        return Ok(new IdResponse("Search saved", ss.Id));
    }

    [HttpDelete("saved-searches/{id}")]
    public async Task<IActionResult> DeleteSavedSearch(long id)
    {
        var ss = await _db.SavedSearches.FirstOrDefaultAsync(s => s.Id == id && s.UserId == Uid);
        if (ss == null) return NotFound();
        _db.SavedSearches.Remove(ss);
        await _db.SaveChangesAsync();
        return Ok(new MessageResponse("Saved search deleted"));
    }
}
