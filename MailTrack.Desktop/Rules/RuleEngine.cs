using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.RegularExpressions;
using MailTrack.Desktop.Models;

namespace MailTrack.Desktop.Rules
{
    public record RuleViolation(DlpRule Rule, string MatchedText);

    public class RuleEngine
    {
        private static readonly Regex CreditCardRegex = new(@"\b(?:\d[ -]*?){13,16}\b", RegexOptions.Compiled);
        private static readonly Regex SsnRegex = new(@"\b\d{3}-\d{2}-\d{4}\b", RegexOptions.Compiled);
        private static readonly Regex EmailRegex = new(@"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b", RegexOptions.Compiled);

        public List<RuleViolation> Evaluate(TrackedEmail email, List<DlpRule> rules)
        {
            return rules
                .Where(r => r.Enabled && MatchesDirection(email, r))
                .Select(r => CheckRule(email, r))
                .Where(v => v != null)
                .ToList()!;
        }

        private bool MatchesDirection(TrackedEmail email, DlpRule rule)
            => rule.Direction == "both" || rule.Direction == email.Direction;

        private RuleViolation? CheckRule(TrackedEmail email, DlpRule rule) => rule.RuleType switch
        {
            "keyword" => CheckKeyword(email, rule),
            "domain" => CheckDomain(email, rule),
            "regex" => CheckRegex(email, rule),
            "attachment_type" => CheckAttachmentType(email, rule),
            "recipient_count" => CheckRecipientCount(email, rule),
            "pii" => CheckPii(email, rule),
            _ => null
        };

        private RuleViolation? CheckKeyword(TrackedEmail email, DlpRule rule)
        {
            var keyword = rule.Pattern.ToLowerInvariant();
            var fields = new[] { email.Subject, email.BodyPreview, email.Sender, email.Recipients };
            foreach (var field in fields)
            {
                if (field.Contains(keyword, StringComparison.OrdinalIgnoreCase))
                    return new RuleViolation(rule, $"Keyword '{keyword}' found");
            }
            return null;
        }

        private RuleViolation? CheckDomain(TrackedEmail email, DlpRule rule)
        {
            var domain = rule.Pattern.ToLowerInvariant();
            var addresses = email.Direction == "outgoing" ? email.Recipients : email.Sender;
            if (addresses.Contains(domain, StringComparison.OrdinalIgnoreCase))
                return new RuleViolation(rule, $"Domain '{domain}' in {email.Direction} email");
            return null;
        }

        private RuleViolation? CheckRegex(TrackedEmail email, DlpRule rule)
        {
            try
            {
                var regex = new Regex(rule.Pattern, RegexOptions.IgnoreCase, TimeSpan.FromSeconds(2));
                var fields = new[] { email.Subject, email.BodyPreview, email.Sender, email.Recipients };
                foreach (var field in fields)
                {
                    var match = regex.Match(field);
                    if (match.Success)
                        return new RuleViolation(rule, $"Pattern matched: '{match.Value}'");
                }
            }
            catch { }
            return null;
        }

        private RuleViolation? CheckAttachmentType(TrackedEmail email, DlpRule rule)
        {
            if (!email.HasAttachment) return null;
            var ext = rule.Pattern.ToLowerInvariant();
            if (email.AttachmentNames.Contains(ext, StringComparison.OrdinalIgnoreCase))
                return new RuleViolation(rule, $"Restricted attachment type '{ext}'");
            return null;
        }

        private RuleViolation? CheckRecipientCount(TrackedEmail email, DlpRule rule)
        {
            if (email.Direction != "outgoing") return null;
            if (!int.TryParse(rule.Pattern, out var max)) return null;
            var count = email.Recipients.Split(',', StringSplitOptions.RemoveEmptyEntries).Length;
            if (count > max)
                return new RuleViolation(rule, $"Recipient count ({count}) exceeds limit ({max})");
            return null;
        }

        private RuleViolation? CheckPii(TrackedEmail email, DlpRule rule)
        {
            var text = email.Subject + " " + email.BodyPreview;
            var piiType = rule.Pattern.ToLowerInvariant();

            if (piiType is "credit_card" or "all")
            {
                var m = CreditCardRegex.Match(text);
                if (m.Success) return new RuleViolation(rule, "Potential credit card number detected");
            }
            if (piiType is "ssn" or "all")
            {
                var m = SsnRegex.Match(text);
                if (m.Success) return new RuleViolation(rule, "Potential SSN detected");
            }
            if (piiType is "email" or "all")
            {
                var matches = EmailRegex.Matches(text);
                if (matches.Count > 3)
                    return new RuleViolation(rule, $"Multiple email addresses ({matches.Count}) detected in body");
            }
            return null;
        }
    }
}
