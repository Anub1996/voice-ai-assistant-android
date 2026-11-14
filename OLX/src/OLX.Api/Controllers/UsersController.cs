using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using OLX.Api.Data;
using OLX.Api.Models;

namespace OLX.Api.Controllers;

[ApiController, Route("api/[controller]")]
public class UsersController : ControllerBase
{
    private readonly AppDbContext _db;

    public UsersController(AppDbContext db) => _db = db;

    [Authorize, HttpGet("me")]
    public async Task<IActionResult> GetProfile()
    {
        var uid = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);
        var user = await _db.Users.FindAsync(uid);
        if (user == null) return NotFound();

        var listingCount = await _db.Listings.CountAsync(l => l.SellerId == uid && l.Status == "active");
        return Ok(new UserProfileDto(user.Id, user.Username, user.FullName, user.Email, user.Phone,
            user.ProfilePhotoUrl, user.Bio, user.City, user.State, user.JoinedAt, user.IsVerified, listingCount, 0));
    }

    [Authorize, HttpPut("me")]
    public async Task<IActionResult> UpdateProfile(UpdateProfileRequest req)
    {
        var uid = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);
        var user = await _db.Users.FindAsync(uid);
        if (user == null) return NotFound();

        if (req.FullName != null) user.FullName = req.FullName;
        if (req.Phone != null) user.Phone = req.Phone;
        if (req.Bio != null) user.Bio = req.Bio;
        if (req.City != null) user.City = req.City;
        if (req.State != null) user.State = req.State;
        await _db.SaveChangesAsync();

        return Ok(new MessageResponse("Profile updated"));
    }

    [HttpGet("{id}")]
    public async Task<IActionResult> GetSeller(int id)
    {
        var user = await _db.Users.FindAsync(id);
        if (user == null) return NotFound();

        return Ok(new SellerDto(user.Id, user.Username, user.FullName, user.ProfilePhotoUrl,
            user.City, user.JoinedAt, user.IsVerified));
    }

    [HttpGet("{id}/listings")]
    public async Task<IActionResult> GetSellerListings(int id, [FromQuery] int page = 1, [FromQuery] int pageSize = 20)
    {
        var q = _db.Listings.Include(l => l.Category).Include(l => l.Seller).Include(l => l.Images)
            .Where(l => l.SellerId == id && l.Status == "active").OrderByDescending(l => l.CreatedAt);

        var total = await q.CountAsync();
        var items = await q.Skip((page - 1) * pageSize).Take(pageSize).ToListAsync();

        var dtos = items.Select(l => new ListingListDto(l.Id, l.Title, l.Slug, l.Price, l.Currency, l.IsNegotiable,
            l.Condition, l.City, l.State, l.Status, l.IsFeatured, l.ViewCount,
            l.Images.OrderBy(i => i.SortOrder).FirstOrDefault()?.Url, l.Category.Name,
            l.Seller.FullName, l.SellerId, l.CreatedAt, false)).ToList();

        return Ok(new PagedResult<ListingListDto>(dtos, total, page, pageSize, (int)Math.Ceiling(total / (double)pageSize)));
    }
}
