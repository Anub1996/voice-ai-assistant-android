using Microsoft.AspNetCore.Authorization;
using Microsoft.AspNetCore.SignalR;

namespace WorkSentinel.Server.Hubs;

[Authorize]
public class MonitorHub : Hub
{
    public async Task JoinDashboard()
        => await Groups.AddToGroupAsync(Context.ConnectionId, "dashboard");

    public async Task LeaveDashboard()
        => await Groups.RemoveFromGroupAsync(Context.ConnectionId, "dashboard");

    public async Task JoinEmployeeWatch(int employeeId)
        => await Groups.AddToGroupAsync(Context.ConnectionId, $"employee-{employeeId}");

    public async Task LeaveEmployeeWatch(int employeeId)
        => await Groups.RemoveFromGroupAsync(Context.ConnectionId, $"employee-{employeeId}");
}
