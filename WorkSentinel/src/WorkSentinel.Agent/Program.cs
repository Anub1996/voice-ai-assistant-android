using WorkSentinel.Agent.Config;
using WorkSentinel.Agent.Services;

var builder = Host.CreateApplicationBuilder(args);

var config = builder.Configuration.GetSection("Agent").Get<AgentConfig>() ?? new AgentConfig();
builder.Services.AddSingleton(config);
builder.Services.AddSingleton<AppMonitorService>();
builder.Services.AddSingleton(new IdleDetector(config.IdleThresholdSeconds));
builder.Services.AddSingleton(new ScreenshotService(config.ScreenshotQuality));
builder.Services.AddSingleton<KeystrokeService>();
builder.Services.AddSingleton<WebMonitorService>();
builder.Services.AddHttpClient<AgentApiClient>();
builder.Services.AddHostedService<MonitorWorker>();
builder.Services.AddWindowsService(opt => opt.ServiceName = "WorkSentinel Agent");

var host = builder.Build();
host.Run();
