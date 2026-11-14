using System.Security.Claims;
using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.Mvc;
using Microsoft.EntityFrameworkCore;
using OLX.Api.Data;
using OLX.Api.Models;
using OLX.Api.Services;

namespace OLX.Api.Controllers;

[ApiController, Route("api/[controller]")]
public class AuthController : ControllerBase
{
    private readonly AppDbContext _db;
    private readonly AuthService _auth;

    public AuthController(AppDbContext db, AuthService auth) { _db = db; _auth = auth; }

    [HttpPost("register")]
    public async Task<IActionResult> Register(RegisterRequest req)
    {
        if (await _db.Users.AnyAsync(u => u.Username == req.Username))
            return Conflict(new MessageResponse("Username already taken"));
        if (await _db.Users.AnyAsync(u => u.Email == req.Email))
            return Conflict(new MessageResponse("Email already registered"));

        var user = new User
        {
            Username = req.Username, Email = req.Email, Phone = req.Phone,
            PasswordHash = BCrypt.Net.BCrypt.HashPassword(req.Password), FullName = req.FullName
        };
        _db.Users.Add(user);
        await _db.SaveChangesAsync();

        var token = _auth.GenerateToken(user.Id, user.Username);
        return Ok(new LoginResponse(token, user.Id, user.Username, user.FullName, null,
            DateTime.UtcNow.AddDays(30).ToString("o")));
    }

    [HttpPost("login")]
    public async Task<IActionResult> Login(LoginRequest req)
    {
        var user = await _db.Users.FirstOrDefaultAsync(u => u.Username == req.Username || u.Email == req.Username);
        if (user == null || !BCrypt.Net.BCrypt.Verify(req.Password, user.PasswordHash))
            return Unauthorized(new MessageResponse("Invalid credentials"));

        user.LastActiveAt = DateTime.UtcNow;
        await _db.SaveChangesAsync();

        var token = _auth.GenerateToken(user.Id, user.Username);
        return Ok(new LoginResponse(token, user.Id, user.Username, user.FullName, user.ProfilePhotoUrl,
            DateTime.UtcNow.AddDays(30).ToString("o")));
    }

    [Authorize, HttpPost("device-token")]
    public async Task<IActionResult> SetDeviceToken(DeviceTokenRequest req)
    {
        var uid = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);
        var user = await _db.Users.FindAsync(uid);
        if (user == null) return NotFound();
        user.DeviceToken = req.Token;
        await _db.SaveChangesAsync();
        return Ok(new MessageResponse("Device token updated"));
    }

    [Authorize, HttpPost("change-password")]
    public async Task<IActionResult> ChangePassword(ChangePasswordRequest req)
    {
        var uid = int.Parse(User.FindFirstValue(ClaimTypes.NameIdentifier)!);
        var user = await _db.Users.FindAsync(uid);
        if (user == null) return NotFound();
        if (!BCrypt.Net.BCrypt.Verify(req.OldPassword, user.PasswordHash))
            return BadRequest(new MessageResponse("Current password is incorrect"));
        user.PasswordHash = BCrypt.Net.BCrypt.HashPassword(req.NewPassword);
        await _db.SaveChangesAsync();
        return Ok(new MessageResponse("Password changed"));
    }
}
