using System.IdentityModel.Tokens.Jwt;
using System.Security.Claims;
using System.Text;
using Microsoft.EntityFrameworkCore;
using Microsoft.IdentityModel.Tokens;
using ZingHR.Api.Data;
using ZingHR.Api.Models;

namespace ZingHR.Api.Services;

public class AuthService
{
    private readonly AppDbContext _db;
    private readonly IConfiguration _config;

    public AuthService(AppDbContext db, IConfiguration config)
    {
        _db = db;
        _config = config;
    }

    public async Task<LoginResponse?> LoginAsync(LoginRequest request)
    {
        var user = await _db.Users.Include(u => u.Employee)
            .FirstOrDefaultAsync(u => u.Username == request.Username && u.IsActive);
        if (user == null || !BCrypt.Net.BCrypt.Verify(request.Password, user.PasswordHash))
            return null;

        user.LastLoginAt = DateTime.UtcNow;
        await _db.SaveChangesAsync();

        var expires = DateTime.UtcNow.AddHours(24);
        var token = GenerateToken(user, expires);
        return new LoginResponse(token, user.Username, user.FullName, user.Role, user.EmployeeId, expires);
    }

    public async Task<LoginResponse?> PinLoginAsync(PinLoginRequest request)
    {
        var user = await _db.Users.Include(u => u.Employee)
            .FirstOrDefaultAsync(u => u.Username == request.Username && u.IsActive && u.QuickPin == request.Pin);
        if (user == null) return null;

        user.LastLoginAt = DateTime.UtcNow;
        await _db.SaveChangesAsync();

        var expires = DateTime.UtcNow.AddHours(24);
        var token = GenerateToken(user, expires);
        return new LoginResponse(token, user.Username, user.FullName, user.Role, user.EmployeeId, expires);
    }

    public async Task<bool> SetPinAsync(int userId, string pin)
    {
        var user = await _db.Users.FindAsync(userId);
        if (user == null) return false;
        user.QuickPin = pin;
        await _db.SaveChangesAsync();
        return true;
    }

    private string GenerateToken(AppUser user, DateTime expires)
    {
        var key = new SymmetricSecurityKey(Encoding.UTF8.GetBytes(
            _config["Jwt:Key"] ?? "ZingHR_DefaultSecret_ChangeInProduction_2024!"));

        var claims = new List<Claim>
        {
            new(ClaimTypes.NameIdentifier, user.Id.ToString()),
            new(ClaimTypes.Name, user.Username),
            new(ClaimTypes.Role, user.Role),
            new("fullName", user.FullName),
            new("email", user.Email)
        };
        if (user.EmployeeId.HasValue)
            claims.Add(new Claim("employeeId", user.EmployeeId.Value.ToString()));

        var creds = new SigningCredentials(key, SecurityAlgorithms.HmacSha256);
        var token = new JwtSecurityToken(
            issuer: _config["Jwt:Issuer"] ?? "ZingHR",
            audience: _config["Jwt:Audience"] ?? "ZingHR",
            claims: claims,
            expires: expires,
            signingCredentials: creds);

        return new JwtSecurityTokenHandler().WriteToken(token);
    }
}
