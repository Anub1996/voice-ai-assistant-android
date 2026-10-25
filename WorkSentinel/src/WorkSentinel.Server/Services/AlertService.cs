using System.Text.RegularExpressions;
using Microsoft.EntityFrameworkCore;
using WorkSentinel.Server.Data;
using WorkSentinel.Server.Models;

namespace WorkSentinel.Server.Services;

public class AlertService
{
    private readonly AppDbContext _db;

    public AlertService(AppDbContext db) => _db = db;

    public async Task EvaluateEmailAsync(EmailLog email, int employeeId)
    {
        var policies = await _db.DlpPolicies
            .Where(p => p.Enabled && p.Category == "email")
            .ToListAsync();

        foreach (var policy in policies)
        {
            string? matched = EvaluatePolicy(policy, email.Subject, email.BodyPreview, email.Sender, email.Recipients, email.AttachmentNames);
            if (matched != null)
            {
                email.AlertTriggered = true;
                _db.Alerts.Add(new Alert
                {
                    EmployeeId = employeeId,
                    PolicyId = policy.Id,
                    Category = "dlp",
                    Severity = policy.Severity,
                    Title = $"DLP: {policy.Name}",
                    Description = matched,
                    Details = $"Email subject: {email.Subject}"
                });
            }
        }
    }

    public async Task EvaluateWebVisitAsync(WebVisit visit, int employeeId)
    {
        var policies = await _db.DlpPolicies
            .Where(p => p.Enabled && p.Category == "web")
            .ToListAsync();

        foreach (var policy in policies)
        {
            if (policy.RuleType == "web_block" &&
                visit.Domain.Contains(policy.Pattern, StringComparison.OrdinalIgnoreCase))
            {
                _db.Alerts.Add(new Alert
                {
                    EmployeeId = employeeId,
                    PolicyId = policy.Id,
                    Category = "policy_violation",
                    Severity = policy.Severity,
                    Title = $"Blocked site visited: {visit.Domain}",
                    Description = $"Policy '{policy.Name}' violated",
                    Details = visit.Url
                });
            }
        }
    }

    public async Task EvaluateFileActivityAsync(FileActivity file, int employeeId)
    {
        var policies = await _db.DlpPolicies
            .Where(p => p.Enabled && p.Category == "file")
            .ToListAsync();

        foreach (var policy in policies)
        {
            if (policy.RuleType == "file_type" &&
                file.FileName.EndsWith(policy.Pattern, StringComparison.OrdinalIgnoreCase))
            {
                _db.Alerts.Add(new Alert
                {
                    EmployeeId = employeeId,
                    PolicyId = policy.Id,
                    Category = "dlp",
                    Severity = policy.Severity,
                    Title = $"Restricted file: {policy.Name}",
                    Description = $"File '{file.FileName}' ({file.Action})",
                    Details = file.FilePath
                });
            }
        }
    }

    private string? EvaluatePolicy(DlpPolicy policy, params string[] fields)
    {
        var combined = string.Join(" ", fields);

        return policy.RuleType switch
        {
            "keyword" when combined.Contains(policy.Pattern, StringComparison.OrdinalIgnoreCase)
                => $"Keyword '{policy.Pattern}' detected",
            "regex" when TryMatchRegex(policy.Pattern, combined, out var m)
                => $"Pattern matched: '{m}'",
            "domain" when combined.Contains(policy.Pattern, StringComparison.OrdinalIgnoreCase)
                => $"Domain '{policy.Pattern}' detected",
            "pii" => CheckPii(policy.Pattern, combined),
            _ => null
        };
    }

    private bool TryMatchRegex(string pattern, string input, out string matched)
    {
        matched = "";
        try
        {
            var m = Regex.Match(input, pattern, RegexOptions.IgnoreCase, TimeSpan.FromSeconds(2));
            if (m.Success) { matched = m.Value; return true; }
        }
        catch { }
        return false;
    }

    private static readonly Regex CreditCardRx = new(@"\b(?:\d[ -]*?){13,16}\b", RegexOptions.Compiled);
    private static readonly Regex SsnRx = new(@"\b\d{3}-\d{2}-\d{4}\b", RegexOptions.Compiled);

    private string? CheckPii(string piiType, string text)
    {
        var t = piiType.ToLowerInvariant();
        if ((t is "credit_card" or "all") && CreditCardRx.IsMatch(text))
            return "Potential credit card number detected";
        if ((t is "ssn" or "all") && SsnRx.IsMatch(text))
            return "Potential SSN detected";
        return null;
    }
}
