using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using OLX.Api.Data;
using OLX.Api.Models;

namespace OLX.Api.Controllers;

[ApiController, Route("api/[controller]"), Authorize]
public class ReportsController : ControllerBase
{
    private readonly AppDbContext _db;

    public ReportsController(AppDbContext db) => _db = db;

    [HttpPost("listings/{listingId}")]
    public async Task<IActionResult> ReportListing(long listingId, ReportRequest req)
    {
        var uid = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);
        if (!await _db.Listings.AnyAsync(l => l.Id == listingId))
            return NotFound();
        if (await _db.Reports.AnyAsync(r => r.ReporterId == uid && r.ListingId == listingId))
            return Conflict(new MessageResponse("Already reported"));

        _db.Reports.Add(new Report { ReporterId = uid, ListingId = listingId, Reason = req.Reason, Description = req.Description });
        await _db.SaveChangesAsync();
        return Ok(new MessageResponse("Report submitted"));
    }
}
