using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using ZingHR.Api.Data;
using ZingHR.Api.Models;
using ZingHR.Api.Services;

namespace ZingHR.Api.Controllers;

[ApiController]
[Route("api/[controller]")]
public class AuthController : ControllerBase
{
    private readonly AuthService _auth;
    private readonly AppDbContext _db;

    public AuthController(AuthService auth, AppDbContext db)
    {
        _auth = auth;
        _db = db;
    }

    [HttpPost("login")]
    public async Task<IActionResult> Login([FromBody] LoginRequest req)
    {
        var result = await _auth.LoginAsync(req);
        if (result == null) return Unauthorized(new { message = "Invalid credentials" });
        return Ok(result);
    }

    [HttpPost("pin-login")]
    public async Task<IActionResult> PinLogin([FromBody] PinLoginRequest req)
    {
        var result = await _auth.PinLoginAsync(req);
        if (result == null) return Unauthorized(new { message = "Invalid PIN" });
        return Ok(result);
    }

    [Authorize]
    [HttpPost("set-pin")]
    public async Task<IActionResult> SetPin([FromBody] SetPinRequest req)
    {
        var userId = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);
        if (req.Pin.Length < 4 || req.Pin.Length > 6) return BadRequest(new { message = "PIN must be 4-6 digits" });
        var ok = await _auth.SetPinAsync(userId, req.Pin);
        return ok ? Ok(new { message = "PIN set" }) : NotFound();
    }

    [Authorize]
    [HttpGet("me")]
    public async Task<IActionResult> Me()
    {
        var userId = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);
        var user = await _db.Users.Include(u => u.Employee).ThenInclude(e => e!.Department)
            .Include(u => u.Employee).ThenInclude(e => e!.Designation)
            .FirstOrDefaultAsync(u => u.Id == userId);
        if (user == null) return NotFound();

        if (user.Employee != null)
        {
            return Ok(new ProfileDto
            {
                Id = user.Employee.Id,
                EmpCode = user.Employee.EmpCode,
                FullName = user.Employee.FullName,
                Email = user.Employee.Email,
                Phone = user.Employee.Phone,
                Gender = user.Employee.Gender,
                Department = user.Employee.Department.Name,
                Designation = user.Employee.Designation.Title,
                ProfilePhotoUrl = user.Employee.ProfilePhotoUrl,
                DateOfJoining = user.Employee.DateOfJoining,
                EmploymentType = user.Employee.EmploymentType,
                Status = user.Employee.Status
            });
        }

        return Ok(new { user.Id, user.Username, user.FullName, user.Email, user.Role });
    }

    [Authorize]
    [HttpPost("change-password")]
    public async Task<IActionResult> ChangePassword([FromBody] ChangePasswordRequest req)
    {
        var userId = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);
        var user = await _db.Users.FindAsync(userId);
        if (user == null) return NotFound();
        if (!BCrypt.Net.BCrypt.Verify(req.OldPassword, user.PasswordHash))
            return BadRequest(new { message = "Current password is incorrect" });
        user.PasswordHash = BCrypt.Net.BCrypt.HashPassword(req.NewPassword);
        await _db.SaveChangesAsync();
        return Ok(new { message = "Password changed" });
    }

    [Authorize]
    [HttpPut("device-token")]
    public async Task<IActionResult> UpdateDeviceToken([FromBody] DeviceTokenRequest req)
    {
        var userId = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);
        var user = await _db.Users.FindAsync(userId);
        if (user == null) return NotFound();
        user.DeviceToken = req.Token;
        await _db.SaveChangesAsync();
        return Ok();
    }
}

public record ChangePasswordRequest(string OldPassword, string NewPassword);
public record DeviceTokenRequest(string Token);
