using System;
using System.Collections.Generic;
using System.IO;
using Microsoft.Data.Sqlite;
using MailTrack.Desktop.Models;

namespace MailTrack.Desktop.Data
{
    public class DatabaseManager
    {
        private readonly string _connectionString;

        public DatabaseManager()
        {
            var dbPath = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "MailTrack", "mailtrack.db");
            Directory.CreateDirectory(Path.GetDirectoryName(dbPath)!);
            _connectionString = $"Data Source={dbPath}";
            InitializeDatabase();
        }

        private void InitializeDatabase()
        {
            using var conn = new SqliteConnection(_connectionString);
            conn.Open();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = @"
                CREATE TABLE IF NOT EXISTS tracked_emails (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    sender TEXT NOT NULL DEFAULT '',
                    recipients TEXT NOT NULL DEFAULT '',
                    cc TEXT NOT NULL DEFAULT '',
                    bcc TEXT NOT NULL DEFAULT '',
                    subject TEXT NOT NULL DEFAULT '',
                    body_preview TEXT NOT NULL DEFAULT '',
                    direction TEXT NOT NULL DEFAULT 'incoming',
                    email_client TEXT NOT NULL DEFAULT '',
                    account TEXT NOT NULL DEFAULT '',
                    has_attachment INTEGER NOT NULL DEFAULT 0,
                    attachment_names TEXT NOT NULL DEFAULT '',
                    attachment_size_bytes INTEGER NOT NULL DEFAULT 0,
                    timestamp TEXT NOT NULL,
                    flagged INTEGER NOT NULL DEFAULT 0,
                    alert_triggered INTEGER NOT NULL DEFAULT 0,
                    alert_rule_id INTEGER
                );

                CREATE TABLE IF NOT EXISTS dlp_rules (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    description TEXT NOT NULL DEFAULT '',
                    rule_type TEXT NOT NULL DEFAULT 'keyword',
                    pattern TEXT NOT NULL,
                    direction TEXT NOT NULL DEFAULT 'both',
                    severity TEXT NOT NULL DEFAULT 'medium',
                    action TEXT NOT NULL DEFAULT 'alert',
                    enabled INTEGER NOT NULL DEFAULT 1,
                    created_at TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS alerts (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    email_id INTEGER NOT NULL,
                    rule_id INTEGER NOT NULL,
                    rule_name TEXT NOT NULL,
                    severity TEXT NOT NULL,
                    description TEXT NOT NULL,
                    acknowledged INTEGER NOT NULL DEFAULT 0,
                    timestamp TEXT NOT NULL
                );

                CREATE TABLE IF NOT EXISTS monitored_accounts (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    name TEXT NOT NULL,
                    account_type TEXT NOT NULL DEFAULT 'imap',
                    host TEXT NOT NULL DEFAULT '',
                    port INTEGER NOT NULL DEFAULT 993,
                    email TEXT NOT NULL DEFAULT '',
                    password TEXT NOT NULL DEFAULT '',
                    use_ssl INTEGER NOT NULL DEFAULT 1,
                    enabled INTEGER NOT NULL DEFAULT 1,
                    poll_interval_seconds INTEGER NOT NULL DEFAULT 60
                );

                CREATE INDEX IF NOT EXISTS idx_emails_timestamp ON tracked_emails(timestamp);
                CREATE INDEX IF NOT EXISTS idx_emails_direction ON tracked_emails(direction);
                CREATE INDEX IF NOT EXISTS idx_alerts_acknowledged ON alerts(acknowledged);
            ";
            cmd.ExecuteNonQuery();
        }

        private SqliteConnection OpenConnection()
        {
            var conn = new SqliteConnection(_connectionString);
            conn.Open();
            return conn;
        }

        // ── Emails ──

        public long InsertEmail(TrackedEmail email)
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = @"INSERT INTO tracked_emails
                (sender, recipients, cc, bcc, subject, body_preview, direction, email_client, account,
                 has_attachment, attachment_names, attachment_size_bytes, timestamp, flagged, alert_triggered, alert_rule_id)
                VALUES ($sender, $recipients, $cc, $bcc, $subject, $body, $dir, $client, $account,
                        $attach, $attach_names, $attach_size, $ts, $flag, $alert, $rule_id);
                SELECT last_insert_rowid();";
            cmd.Parameters.AddWithValue("$sender", email.Sender);
            cmd.Parameters.AddWithValue("$recipients", email.Recipients);
            cmd.Parameters.AddWithValue("$cc", email.Cc);
            cmd.Parameters.AddWithValue("$bcc", email.Bcc);
            cmd.Parameters.AddWithValue("$subject", email.Subject);
            cmd.Parameters.AddWithValue("$body", email.BodyPreview);
            cmd.Parameters.AddWithValue("$dir", email.Direction);
            cmd.Parameters.AddWithValue("$client", email.EmailClient);
            cmd.Parameters.AddWithValue("$account", email.Account);
            cmd.Parameters.AddWithValue("$attach", email.HasAttachment ? 1 : 0);
            cmd.Parameters.AddWithValue("$attach_names", email.AttachmentNames);
            cmd.Parameters.AddWithValue("$attach_size", email.AttachmentSizeBytes);
            cmd.Parameters.AddWithValue("$ts", email.Timestamp.ToString("o"));
            cmd.Parameters.AddWithValue("$flag", email.Flagged ? 1 : 0);
            cmd.Parameters.AddWithValue("$alert", email.AlertTriggered ? 1 : 0);
            cmd.Parameters.AddWithValue("$rule_id", (object?)email.AlertRuleId ?? DBNull.Value);
            return (long)cmd.ExecuteScalar()!;
        }

        public void UpdateEmailAlert(long emailId, long ruleId)
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = "UPDATE tracked_emails SET alert_triggered = 1, alert_rule_id = $rid WHERE id = $id";
            cmd.Parameters.AddWithValue("$id", emailId);
            cmd.Parameters.AddWithValue("$rid", ruleId);
            cmd.ExecuteNonQuery();
        }

        public void ToggleEmailFlag(long emailId)
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = "UPDATE tracked_emails SET flagged = NOT flagged WHERE id = $id";
            cmd.Parameters.AddWithValue("$id", emailId);
            cmd.ExecuteNonQuery();
        }

        public List<TrackedEmail> GetEmails(string? direction = null, string? search = null, bool? alertsOnly = null)
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            var where = new List<string>();
            if (direction != null) { where.Add("direction = $dir"); cmd.Parameters.AddWithValue("$dir", direction); }
            if (alertsOnly == true) where.Add("alert_triggered = 1");
            if (!string.IsNullOrWhiteSpace(search))
            {
                where.Add("(sender LIKE $q OR recipients LIKE $q OR subject LIKE $q OR body_preview LIKE $q)");
                cmd.Parameters.AddWithValue("$q", $"%{search}%");
            }
            var clause = where.Count > 0 ? "WHERE " + string.Join(" AND ", where) : "";
            cmd.CommandText = $"SELECT * FROM tracked_emails {clause} ORDER BY timestamp DESC LIMIT 500";
            return ReadEmails(cmd);
        }

        public TrackedEmail? GetEmailById(long id)
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = "SELECT * FROM tracked_emails WHERE id = $id";
            cmd.Parameters.AddWithValue("$id", id);
            var list = ReadEmails(cmd);
            return list.Count > 0 ? list[0] : null;
        }

        private List<TrackedEmail> ReadEmails(SqliteCommand cmd)
        {
            var list = new List<TrackedEmail>();
            using var reader = cmd.ExecuteReader();
            while (reader.Read())
            {
                list.Add(new TrackedEmail
                {
                    Id = reader.GetInt64(reader.GetOrdinal("id")),
                    Sender = reader.GetString(reader.GetOrdinal("sender")),
                    Recipients = reader.GetString(reader.GetOrdinal("recipients")),
                    Cc = reader.GetString(reader.GetOrdinal("cc")),
                    Bcc = reader.GetString(reader.GetOrdinal("bcc")),
                    Subject = reader.GetString(reader.GetOrdinal("subject")),
                    BodyPreview = reader.GetString(reader.GetOrdinal("body_preview")),
                    Direction = reader.GetString(reader.GetOrdinal("direction")),
                    EmailClient = reader.GetString(reader.GetOrdinal("email_client")),
                    Account = reader.GetString(reader.GetOrdinal("account")),
                    HasAttachment = reader.GetInt64(reader.GetOrdinal("has_attachment")) == 1,
                    AttachmentNames = reader.GetString(reader.GetOrdinal("attachment_names")),
                    AttachmentSizeBytes = reader.GetInt64(reader.GetOrdinal("attachment_size_bytes")),
                    Timestamp = DateTime.Parse(reader.GetString(reader.GetOrdinal("timestamp"))),
                    Flagged = reader.GetInt64(reader.GetOrdinal("flagged")) == 1,
                    AlertTriggered = reader.GetInt64(reader.GetOrdinal("alert_triggered")) == 1,
                    AlertRuleId = reader.IsDBNull(reader.GetOrdinal("alert_rule_id")) ? null : reader.GetInt64(reader.GetOrdinal("alert_rule_id"))
                });
            }
            return list;
        }

        // ── Rules ──

        public long InsertRule(DlpRule rule)
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = @"INSERT INTO dlp_rules (name, description, rule_type, pattern, direction, severity, action, enabled, created_at)
                VALUES ($name, $desc, $type, $pat, $dir, $sev, $act, $en, $ts); SELECT last_insert_rowid();";
            cmd.Parameters.AddWithValue("$name", rule.Name);
            cmd.Parameters.AddWithValue("$desc", rule.Description);
            cmd.Parameters.AddWithValue("$type", rule.RuleType);
            cmd.Parameters.AddWithValue("$pat", rule.Pattern);
            cmd.Parameters.AddWithValue("$dir", rule.Direction);
            cmd.Parameters.AddWithValue("$sev", rule.Severity);
            cmd.Parameters.AddWithValue("$act", rule.Action);
            cmd.Parameters.AddWithValue("$en", rule.Enabled ? 1 : 0);
            cmd.Parameters.AddWithValue("$ts", rule.CreatedAt.ToString("o"));
            return (long)cmd.ExecuteScalar()!;
        }

        public void UpdateRule(DlpRule rule)
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = @"UPDATE dlp_rules SET name=$name, description=$desc, rule_type=$type,
                pattern=$pat, direction=$dir, severity=$sev, action=$act, enabled=$en WHERE id=$id";
            cmd.Parameters.AddWithValue("$id", rule.Id);
            cmd.Parameters.AddWithValue("$name", rule.Name);
            cmd.Parameters.AddWithValue("$desc", rule.Description);
            cmd.Parameters.AddWithValue("$type", rule.RuleType);
            cmd.Parameters.AddWithValue("$pat", rule.Pattern);
            cmd.Parameters.AddWithValue("$dir", rule.Direction);
            cmd.Parameters.AddWithValue("$sev", rule.Severity);
            cmd.Parameters.AddWithValue("$act", rule.Action);
            cmd.Parameters.AddWithValue("$en", rule.Enabled ? 1 : 0);
            cmd.ExecuteNonQuery();
        }

        public void DeleteRule(long id)
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = "DELETE FROM dlp_rules WHERE id = $id";
            cmd.Parameters.AddWithValue("$id", id);
            cmd.ExecuteNonQuery();
        }

        public List<DlpRule> GetAllRules()
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = "SELECT * FROM dlp_rules ORDER BY created_at DESC";
            return ReadRules(cmd);
        }

        public List<DlpRule> GetEnabledRules()
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = "SELECT * FROM dlp_rules WHERE enabled = 1";
            return ReadRules(cmd);
        }

        private List<DlpRule> ReadRules(SqliteCommand cmd)
        {
            var list = new List<DlpRule>();
            using var reader = cmd.ExecuteReader();
            while (reader.Read())
            {
                list.Add(new DlpRule
                {
                    Id = reader.GetInt64(reader.GetOrdinal("id")),
                    Name = reader.GetString(reader.GetOrdinal("name")),
                    Description = reader.GetString(reader.GetOrdinal("description")),
                    RuleType = reader.GetString(reader.GetOrdinal("rule_type")),
                    Pattern = reader.GetString(reader.GetOrdinal("pattern")),
                    Direction = reader.GetString(reader.GetOrdinal("direction")),
                    Severity = reader.GetString(reader.GetOrdinal("severity")),
                    Action = reader.GetString(reader.GetOrdinal("action")),
                    Enabled = reader.GetInt64(reader.GetOrdinal("enabled")) == 1,
                    CreatedAt = DateTime.Parse(reader.GetString(reader.GetOrdinal("created_at")))
                });
            }
            return list;
        }

        // ── Alerts ──

        public long InsertAlert(Alert alert)
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = @"INSERT INTO alerts (email_id, rule_id, rule_name, severity, description, acknowledged, timestamp)
                VALUES ($eid, $rid, $rname, $sev, $desc, 0, $ts); SELECT last_insert_rowid();";
            cmd.Parameters.AddWithValue("$eid", alert.EmailId);
            cmd.Parameters.AddWithValue("$rid", alert.RuleId);
            cmd.Parameters.AddWithValue("$rname", alert.RuleName);
            cmd.Parameters.AddWithValue("$sev", alert.Severity);
            cmd.Parameters.AddWithValue("$desc", alert.Description);
            cmd.Parameters.AddWithValue("$ts", alert.Timestamp.ToString("o"));
            return (long)cmd.ExecuteScalar()!;
        }

        public List<Alert> GetAlerts(bool unacknowledgedOnly = false)
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = unacknowledgedOnly
                ? "SELECT * FROM alerts WHERE acknowledged = 0 ORDER BY timestamp DESC"
                : "SELECT * FROM alerts ORDER BY timestamp DESC LIMIT 200";
            return ReadAlerts(cmd);
        }

        public void AcknowledgeAlert(long id)
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = "UPDATE alerts SET acknowledged = 1 WHERE id = $id";
            cmd.Parameters.AddWithValue("$id", id);
            cmd.ExecuteNonQuery();
        }

        public void AcknowledgeAllAlerts()
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = "UPDATE alerts SET acknowledged = 1";
            cmd.ExecuteNonQuery();
        }

        private List<Alert> ReadAlerts(SqliteCommand cmd)
        {
            var list = new List<Alert>();
            using var reader = cmd.ExecuteReader();
            while (reader.Read())
            {
                list.Add(new Alert
                {
                    Id = reader.GetInt64(reader.GetOrdinal("id")),
                    EmailId = reader.GetInt64(reader.GetOrdinal("email_id")),
                    RuleId = reader.GetInt64(reader.GetOrdinal("rule_id")),
                    RuleName = reader.GetString(reader.GetOrdinal("rule_name")),
                    Severity = reader.GetString(reader.GetOrdinal("severity")),
                    Description = reader.GetString(reader.GetOrdinal("description")),
                    Acknowledged = reader.GetInt64(reader.GetOrdinal("acknowledged")) == 1,
                    Timestamp = DateTime.Parse(reader.GetString(reader.GetOrdinal("timestamp")))
                });
            }
            return list;
        }

        // ── Accounts ──

        public long InsertAccount(MonitoredAccount account)
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = @"INSERT INTO monitored_accounts (name, account_type, host, port, email, password, use_ssl, enabled, poll_interval_seconds)
                VALUES ($name, $type, $host, $port, $email, $pass, $ssl, $en, $poll); SELECT last_insert_rowid();";
            cmd.Parameters.AddWithValue("$name", account.Name);
            cmd.Parameters.AddWithValue("$type", account.AccountType);
            cmd.Parameters.AddWithValue("$host", account.Host);
            cmd.Parameters.AddWithValue("$port", account.Port);
            cmd.Parameters.AddWithValue("$email", account.Email);
            cmd.Parameters.AddWithValue("$pass", account.Password);
            cmd.Parameters.AddWithValue("$ssl", account.UseSsl ? 1 : 0);
            cmd.Parameters.AddWithValue("$en", account.Enabled ? 1 : 0);
            cmd.Parameters.AddWithValue("$poll", account.PollIntervalSeconds);
            return (long)cmd.ExecuteScalar()!;
        }

        public void DeleteAccount(long id)
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = "DELETE FROM monitored_accounts WHERE id = $id";
            cmd.Parameters.AddWithValue("$id", id);
            cmd.ExecuteNonQuery();
        }

        public List<MonitoredAccount> GetAccounts()
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = "SELECT * FROM monitored_accounts";
            var list = new List<MonitoredAccount>();
            using var reader = cmd.ExecuteReader();
            while (reader.Read())
            {
                list.Add(new MonitoredAccount
                {
                    Id = reader.GetInt64(reader.GetOrdinal("id")),
                    Name = reader.GetString(reader.GetOrdinal("name")),
                    AccountType = reader.GetString(reader.GetOrdinal("account_type")),
                    Host = reader.GetString(reader.GetOrdinal("host")),
                    Port = reader.GetInt32(reader.GetOrdinal("port")),
                    Email = reader.GetString(reader.GetOrdinal("email")),
                    Password = reader.GetString(reader.GetOrdinal("password")),
                    UseSsl = reader.GetInt64(reader.GetOrdinal("use_ssl")) == 1,
                    Enabled = reader.GetInt64(reader.GetOrdinal("enabled")) == 1,
                    PollIntervalSeconds = reader.GetInt32(reader.GetOrdinal("poll_interval_seconds"))
                });
            }
            return list;
        }

        // ── Stats ──

        public (int total, int incoming, int outgoing, int alerts, int activeRules, int pendingAlerts) GetStats()
        {
            using var conn = OpenConnection();
            using var cmd = conn.CreateCommand();
            cmd.CommandText = @"
                SELECT
                    (SELECT COUNT(*) FROM tracked_emails) as total,
                    (SELECT COUNT(*) FROM tracked_emails WHERE direction='incoming') as incoming,
                    (SELECT COUNT(*) FROM tracked_emails WHERE direction='outgoing') as outgoing,
                    (SELECT COUNT(*) FROM tracked_emails WHERE alert_triggered=1) as alerts,
                    (SELECT COUNT(*) FROM dlp_rules WHERE enabled=1) as active_rules,
                    (SELECT COUNT(*) FROM alerts WHERE acknowledged=0) as pending_alerts";
            using var reader = cmd.ExecuteReader();
            reader.Read();
            return (
                (int)(long)reader["total"],
                (int)(long)reader["incoming"],
                (int)(long)reader["outgoing"],
                (int)(long)reader["alerts"],
                (int)(long)reader["active_rules"],
                (int)(long)reader["pending_alerts"]
            );
        }
    }
}
