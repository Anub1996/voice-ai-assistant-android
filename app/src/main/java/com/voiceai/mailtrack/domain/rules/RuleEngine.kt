package com.voiceai.mailtrack.domain.rules

import com.voiceai.mailtrack.data.entity.DlpRule
import com.voiceai.mailtrack.data.entity.TrackedEmail

data class RuleViolation(val rule: DlpRule, val matchedText: String)

class RuleEngine {

    fun evaluate(email: TrackedEmail, rules: List<DlpRule>): List<RuleViolation> {
        return rules.filter { it.enabled }.mapNotNull { rule ->
            if (!matchesDirection(email, rule)) return@mapNotNull null
            checkRule(email, rule)
        }
    }

    private fun matchesDirection(email: TrackedEmail, rule: DlpRule): Boolean {
        return rule.direction == "both" || rule.direction == email.direction
    }

    private fun checkRule(email: TrackedEmail, rule: DlpRule): RuleViolation? {
        return when (rule.ruleType) {
            "keyword" -> checkKeyword(email, rule)
            "domain" -> checkDomain(email, rule)
            "regex" -> checkRegex(email, rule)
            "attachment_type" -> checkAttachmentType(email, rule)
            "recipient_count" -> checkRecipientCount(email, rule)
            else -> null
        }
    }

    private fun checkKeyword(email: TrackedEmail, rule: DlpRule): RuleViolation? {
        val keyword = rule.pattern.lowercase()
        val fields = listOf(email.subject, email.bodyPreview, email.sender, email.recipients)
        for (field in fields) {
            if (field.lowercase().contains(keyword)) {
                return RuleViolation(rule, "Keyword '$keyword' found in email")
            }
        }
        return null
    }

    private fun checkDomain(email: TrackedEmail, rule: DlpRule): RuleViolation? {
        val domain = rule.pattern.lowercase()
        val addresses = if (email.direction == "outgoing") email.recipients else email.sender
        if (addresses.lowercase().contains(domain)) {
            return RuleViolation(rule, "Domain '$domain' detected in ${email.direction} email")
        }
        return null
    }

    private fun checkRegex(email: TrackedEmail, rule: DlpRule): RuleViolation? {
        val regex = try {
            Regex(rule.pattern, RegexOption.IGNORE_CASE)
        } catch (_: Exception) {
            return null
        }
        val fields = listOf(email.subject, email.bodyPreview, email.sender, email.recipients)
        for (field in fields) {
            val match = regex.find(field)
            if (match != null) {
                return RuleViolation(rule, "Pattern matched: '${match.value}'")
            }
        }
        return null
    }

    private fun checkAttachmentType(email: TrackedEmail, rule: DlpRule): RuleViolation? {
        if (!email.hasAttachment) return null
        val ext = rule.pattern.lowercase()
        val attachments = email.attachmentNames?.lowercase() ?: return null
        if (attachments.contains(ext)) {
            return RuleViolation(rule, "Restricted attachment type '$ext' detected")
        }
        return null
    }

    private fun checkRecipientCount(email: TrackedEmail, rule: DlpRule): RuleViolation? {
        if (email.direction != "outgoing") return null
        val maxRecipients = rule.pattern.toIntOrNull() ?: return null
        val recipientCount = email.recipients.split(",").size
        if (recipientCount > maxRecipients) {
            return RuleViolation(rule, "Recipient count ($recipientCount) exceeds limit ($maxRecipients)")
        }
        return null
    }
}
