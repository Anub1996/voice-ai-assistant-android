using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using MailKit;
using MailKit.Net.Imap;
using MailKit.Search;
using MailTrack.Desktop.Data;
using MailTrack.Desktop.Models;
using MailTrack.Desktop.Rules;

namespace MailTrack.Desktop.Services
{
    public class ImapMonitorService
    {
        private readonly DatabaseManager _db;
        private readonly RuleEngine _ruleEngine = new();
        private readonly Dictionary<long, CancellationTokenSource> _monitors = new();

        public event Action<TrackedEmail>? EmailCaptured;
        public event Action<Alert>? AlertRaised;
        public event Action<string>? StatusChanged;

        public ImapMonitorService(DatabaseManager db)
        {
            _db = db;
        }

        public void StartMonitoring(MonitoredAccount account)
        {
            StopMonitoring(account.Id);
            var cts = new CancellationTokenSource();
            _monitors[account.Id] = cts;
            _ = MonitorLoop(account, cts.Token);
        }

        public void StopMonitoring(long accountId)
        {
            if (_monitors.TryGetValue(accountId, out var cts))
            {
                cts.Cancel();
                cts.Dispose();
                _monitors.Remove(accountId);
            }
        }

        public void StopAll()
        {
            foreach (var cts in _monitors.Values) { cts.Cancel(); cts.Dispose(); }
            _monitors.Clear();
        }

        private async Task MonitorLoop(MonitoredAccount account, CancellationToken ct)
        {
            var lastSeenUid = UniqueId.Invalid;
            StatusChanged?.Invoke($"Connecting to {account.Email}...");

            while (!ct.IsCancellationRequested)
            {
                try
                {
                    using var client = new ImapClient();
                    await client.ConnectAsync(account.Host, account.Port, account.UseSsl, ct);
                    await client.AuthenticateAsync(account.Email, account.Password, ct);
                    StatusChanged?.Invoke($"Connected: {account.Email}");

                    var inbox = client.Inbox;
                    await inbox.OpenAsync(FolderAccess.ReadOnly, ct);

                    // On first pass, get the latest UID as baseline
                    if (!lastSeenUid.IsValid && inbox.Count > 0)
                    {
                        var uids = await inbox.SearchAsync(SearchQuery.All, ct);
                        if (uids.Count > 0) lastSeenUid = uids[^1];
                    }

                    while (!ct.IsCancellationRequested)
                    {
                        var uids = lastSeenUid.IsValid
                            ? await inbox.SearchAsync(SearchQuery.Uids(new UniqueIdRange(new UniqueId(lastSeenUid.Id + 1), UniqueId.MaxValue)), ct)
                            : await inbox.SearchAsync(SearchQuery.All, ct);

                        foreach (var uid in uids)
                        {
                            var message = await inbox.GetMessageAsync(uid, ct);
                            var attachments = message.Attachments.ToList();

                            var email = new TrackedEmail
                            {
                                Sender = message.From?.ToString() ?? "",
                                Recipients = message.To?.ToString() ?? "",
                                Cc = message.Cc?.ToString() ?? "",
                                Subject = message.Subject ?? "",
                                BodyPreview = (message.TextBody ?? message.HtmlBody ?? "").Length > 500
                                    ? (message.TextBody ?? message.HtmlBody ?? "")[..500]
                                    : message.TextBody ?? message.HtmlBody ?? "",
                                Direction = "incoming",
                                EmailClient = "IMAP",
                                Account = account.Email,
                                HasAttachment = attachments.Count > 0,
                                AttachmentNames = string.Join(", ", attachments.Select(a => a.ContentDisposition?.FileName ?? "unnamed")),
                                Timestamp = message.Date.UtcDateTime
                            };

                            var emailId = _db.InsertEmail(email);
                            email.Id = emailId;

                            var rules = _db.GetEnabledRules();
                            var violations = _ruleEngine.Evaluate(email, rules);
                            if (violations.Count > 0)
                            {
                                _db.UpdateEmailAlert(emailId, violations[0].Rule.Id);
                                email.AlertTriggered = true;
                                foreach (var v in violations)
                                {
                                    var alert = new Alert
                                    {
                                        EmailId = emailId,
                                        RuleId = v.Rule.Id,
                                        RuleName = v.Rule.Name,
                                        Severity = v.Rule.Severity,
                                        Description = v.MatchedText
                                    };
                                    alert.Id = _db.InsertAlert(alert);
                                    AlertRaised?.Invoke(alert);
                                }
                            }

                            EmailCaptured?.Invoke(email);
                            lastSeenUid = uid;
                        }

                        // Also check Sent folder for outgoing
                        await CheckSentFolder(client, account, ct);

                        await Task.Delay(TimeSpan.FromSeconds(account.PollIntervalSeconds), ct);
                        await inbox.OpenAsync(FolderAccess.ReadOnly, ct);
                    }

                    await client.DisconnectAsync(true, ct);
                }
                catch (OperationCanceledException) { break; }
                catch (Exception ex)
                {
                    StatusChanged?.Invoke($"Error ({account.Email}): {ex.Message}. Retrying in 30s...");
                    try { await Task.Delay(30_000, ct); } catch { break; }
                }
            }

            StatusChanged?.Invoke($"Stopped: {account.Email}");
        }

        private async Task CheckSentFolder(ImapClient client, MonitoredAccount account, CancellationToken ct)
        {
            try
            {
                var sent = client.GetFolder(SpecialFolder.Sent);
                if (sent == null) return;
                await sent.OpenAsync(FolderAccess.ReadOnly, ct);

                var recent = await sent.SearchAsync(
                    SearchQuery.DeliveredAfter(DateTime.UtcNow.AddMinutes(-2)), ct);

                foreach (var uid in recent.TakeLast(5))
                {
                    var message = await sent.GetMessageAsync(uid, ct);
                    var attachments = message.Attachments.ToList();

                    var email = new TrackedEmail
                    {
                        Sender = account.Email,
                        Recipients = message.To?.ToString() ?? "",
                        Cc = message.Cc?.ToString() ?? "",
                        Bcc = message.Bcc?.ToString() ?? "",
                        Subject = message.Subject ?? "",
                        BodyPreview = (message.TextBody ?? message.HtmlBody ?? "").Length > 500
                            ? (message.TextBody ?? message.HtmlBody ?? "")[..500]
                            : message.TextBody ?? message.HtmlBody ?? "",
                        Direction = "outgoing",
                        EmailClient = "IMAP",
                        Account = account.Email,
                        HasAttachment = attachments.Count > 0,
                        AttachmentNames = string.Join(", ", attachments.Select(a => a.ContentDisposition?.FileName ?? "unnamed")),
                        Timestamp = message.Date.UtcDateTime
                    };

                    var emailId = _db.InsertEmail(email);
                    email.Id = emailId;

                    var rules = _db.GetEnabledRules();
                    var violations = _ruleEngine.Evaluate(email, rules);
                    if (violations.Count > 0)
                    {
                        _db.UpdateEmailAlert(emailId, violations[0].Rule.Id);
                        email.AlertTriggered = true;
                        foreach (var v in violations)
                        {
                            var alert = new Alert
                            {
                                EmailId = emailId,
                                RuleId = v.Rule.Id,
                                RuleName = v.Rule.Name,
                                Severity = v.Rule.Severity,
                                Description = v.MatchedText
                            };
                            alert.Id = _db.InsertAlert(alert);
                            AlertRaised?.Invoke(alert);
                        }
                    }

                    EmailCaptured?.Invoke(email);
                }
            }
            catch { }
        }
    }
}
