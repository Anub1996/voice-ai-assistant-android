using System.Net.Http.Json;
using System.Text.Json;
using Microsoft.Extensions.Logging;
using WorkSentinel.Agent.Config;

namespace WorkSentinel.Agent.Services;

public class AgentApiClient
{
    private readonly HttpClient _http;
    private readonly AgentConfig _config;
    private readonly ILogger<AgentApiClient> _logger;

    public AgentApiClient(HttpClient http, AgentConfig config, ILogger<AgentApiClient> logger)
    {
        _http = http;
        _config = config;
        _http.BaseAddress = new Uri(config.ServerUrl);
        _logger = logger;
    }

    public async Task<bool> RegisterAsync()
    {
        try
        {
            var res = await _http.PostAsJsonAsync("/api/agent/register", new
            {
                employeeId = _config.EmployeeId,
                fullName = _config.EmployeeName,
                email = _config.Email,
                department = _config.Department,
                computerName = _config.ComputerName
            });
            return res.IsSuccessStatusCode;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Failed to register agent");
            return false;
        }
    }

    public async Task SendHeartbeatAsync(string status, string activeWindow, string activeApp)
    {
        try
        {
            await _http.PostAsJsonAsync("/api/agent/heartbeat", new
            {
                computerName = _config.ComputerName,
                status,
                activeWindow,
                activeApp
            });
        }
        catch (Exception ex)
        {
            _logger.LogWarning(ex, "Heartbeat failed");
        }
    }

    public async Task<bool> SendDataAsync(object payload)
    {
        try
        {
            var res = await _http.PostAsJsonAsync("/api/agent/data", payload);
            return res.IsSuccessStatusCode;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Data sync failed");
            return false;
        }
    }

    public async Task<bool> UploadScreenshotAsync(string computerName, byte[] imageData)
    {
        try
        {
            using var content = new MultipartFormDataContent();
            content.Add(new StringContent(computerName), "computerName");
            content.Add(new ByteArrayContent(imageData), "file", $"screenshot_{DateTime.UtcNow:yyyyMMddHHmmss}.jpg");
            var res = await _http.PostAsync("/api/agent/screenshot", content);
            return res.IsSuccessStatusCode;
        }
        catch (Exception ex)
        {
            _logger.LogError(ex, "Screenshot upload failed");
            return false;
        }
    }
}
