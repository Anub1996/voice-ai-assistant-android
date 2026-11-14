using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using OLX.Api.Data;
using OLX.Api.Models;

namespace OLX.Api.Controllers;

[ApiController, Route("api/[controller]")]
public class CategoriesController : ControllerBase
{
    private readonly AppDbContext _db;

    public CategoriesController(AppDbContext db) => _db = db;

    [HttpGet]
    public async Task<IActionResult> GetAll()
    {
        var cats = await _db.Categories.Include(c => c.Children).Where(c => c.ParentId == null)
            .OrderBy(c => c.SortOrder).ToListAsync();

        return Ok(cats.Select(c => new CategoryDto(c.Id, c.Name, c.Slug, c.Icon, null,
            c.Children.OrderBy(ch => ch.SortOrder).Select(ch => new CategoryDto(ch.Id, ch.Name, ch.Slug, ch.Icon, ch.ParentId, null)).ToList())));
    }

    [HttpGet("{id}")]
    public async Task<IActionResult> GetById(int id)
    {
        var cat = await _db.Categories.Include(c => c.Children).FirstOrDefaultAsync(c => c.Id == id);
        if (cat == null) return NotFound();

        return Ok(new CategoryDto(cat.Id, cat.Name, cat.Slug, cat.Icon, cat.ParentId,
            cat.Children.OrderBy(ch => ch.SortOrder).Select(ch => new CategoryDto(ch.Id, ch.Name, ch.Slug, ch.Icon, ch.ParentId, null)).ToList()));
    }

    [HttpGet("{id}/listings")]
    public async Task<IActionResult> GetListings(int id, [FromQuery] int page = 1, [FromQuery] int pageSize = 20)
    {
        var catIds = new List<int> { id };
        catIds.AddRange(await _db.Categories.Where(c => c.ParentId == id).Select(c => c.Id).ToListAsync());

        var q = _db.Listings.Include(l => l.Category).Include(l => l.Seller).Include(l => l.Images)
            .Where(l => catIds.Contains(l.CategoryId) && l.Status == "active")
            .OrderByDescending(l => l.IsFeatured).ThenByDescending(l => l.CreatedAt);

        var total = await q.CountAsync();
        var items = await q.Skip((page - 1) * pageSize).Take(pageSize).ToListAsync();

        var dtos = items.Select(l => new ListingListDto(l.Id, l.Title, l.Slug, l.Price, l.Currency, l.IsNegotiable,
            l.Condition, l.City, l.State, l.Status, l.IsFeatured, l.ViewCount,
            l.Images.OrderBy(i => i.SortOrder).FirstOrDefault()?.Url, l.Category.Name,
            l.Seller.FullName, l.SellerId, l.CreatedAt, false)).ToList();

        return Ok(new PagedResult<ListingListDto>(dtos, total, page, pageSize, (int)Math.Ceiling(total / (double)pageSize)));
    }
}
