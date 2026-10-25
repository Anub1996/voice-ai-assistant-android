using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using WorkSentinel.Server.Data;
using WorkSentinel.Server.Models;

namespace WorkSentinel.Server.Controllers;

[ApiController]
[Route("api/[controller]")]
[Authorize]
public class PoliciesController : ControllerBase
{
    private readonly AppDbContext _db;

    public PoliciesController(AppDbContext db) => _db = db;

    [HttpGet]
    public async Task<IActionResult> GetAll()
    {
        var policies = await _db.DlpPolicies.OrderByDescending(p => p.CreatedAt).ToListAsync();
        return Ok(policies);
    }

    [HttpGet("{id}")]
    public async Task<IActionResult> Get(int id)
    {
        var policy = await _db.DlpPolicies.FindAsync(id);
        if (policy == null) return NotFound();
        return Ok(policy);
    }

    [HttpPost]
    public async Task<IActionResult> Create([FromBody] DlpPolicy policy)
    {
        policy.CreatedAt = DateTime.UtcNow;
        _db.DlpPolicies.Add(policy);
        await _db.SaveChangesAsync();
        return CreatedAtAction(nameof(Get), new { id = policy.Id }, policy);
    }

    [HttpPut("{id}")]
    public async Task<IActionResult> Update(int id, [FromBody] DlpPolicy updated)
    {
        var policy = await _db.DlpPolicies.FindAsync(id);
        if (policy == null) return NotFound();

        policy.Name = updated.Name;
        policy.Description = updated.Description;
        policy.Category = updated.Category;
        policy.RuleType = updated.RuleType;
        policy.Pattern = updated.Pattern;
        policy.Severity = updated.Severity;
        policy.Action = updated.Action;
        policy.Enabled = updated.Enabled;
        policy.UpdatedAt = DateTime.UtcNow;

        await _db.SaveChangesAsync();
        return Ok(policy);
    }

    [HttpDelete("{id}")]
    public async Task<IActionResult> Delete(int id)
    {
        var policy = await _db.DlpPolicies.FindAsync(id);
        if (policy == null) return NotFound();
        _db.DlpPolicies.Remove(policy);
        await _db.SaveChangesAsync();
        return NoContent();
    }

    [HttpPut("{id}/toggle")]
    public async Task<IActionResult> Toggle(int id)
    {
        var policy = await _db.DlpPolicies.FindAsync(id);
        if (policy == null) return NotFound();
        policy.Enabled = !policy.Enabled;
        policy.UpdatedAt = DateTime.UtcNow;
        await _db.SaveChangesAsync();
        return Ok(policy);
    }
}
