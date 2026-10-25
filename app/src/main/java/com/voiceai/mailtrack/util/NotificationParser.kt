package com.voiceai.mailtrack.util

import android.app.Notification
import android.service.notification.StatusBarNotification

object NotificationParser {

    data class ParsedEmail(
        val sender: String,
        val subject: String,
        val bodyPreview: String,
        val recipients: String,
        val hasAttachment: Boolean,
        val emailClient: String,
        val direction: String
    )

    fun parse(sbn: StatusBarNotification): ParsedEmail? {
        val packageName = sbn.packageName
        if (!EmailClientDetector.isEmailPackage(packageName)) return null

        val notification = sbn.notification
        val extras = notification.extras

        val title = extras.getCharSequence(Notification.EXTRA_TITLE)?.toString() ?: ""
        val text = extras.getCharSequence(Notification.EXTRA_TEXT)?.toString() ?: ""
        val bigText = extras.getCharSequence(Notification.EXTRA_BIG_TEXT)?.toString() ?: ""
        val subText = extras.getCharSequence(Notification.EXTRA_SUB_TEXT)?.toString() ?: ""
        val infoText = extras.getCharSequence(Notification.EXTRA_INFO_TEXT)?.toString() ?: ""

        val bodyPreview = bigText.ifBlank { text }
        val clientName = EmailClientDetector.getClientName(packageName)

        val sender = extractSender(title, clientName)
        val subject = extractSubject(title, text, clientName)
        val hasAttachment = detectAttachment(bodyPreview, subText, infoText)

        return ParsedEmail(
            sender = sender,
            subject = subject,
            bodyPreview = bodyPreview.take(500),
            recipients = "",
            hasAttachment = hasAttachment,
            emailClient = clientName,
            direction = "incoming"
        )
    }

    private fun extractSender(title: String, client: String): String {
        return when (client) {
            "Gmail" -> {
                if (title.contains(":")) title.substringBefore(":").trim()
                else title.trim()
            }
            "Outlook" -> {
                if (title.contains(" - ")) title.substringBefore(" - ").trim()
                else title.trim()
            }
            else -> title.trim()
        }
    }

    private fun extractSubject(title: String, text: String, client: String): String {
        return when (client) {
            "Gmail" -> {
                if (title.contains(":")) title.substringAfter(":").trim()
                else text.trim()
            }
            "Outlook" -> {
                if (title.contains(" - ")) title.substringAfter(" - ").trim()
                else text.trim()
            }
            else -> text.trim()
        }
    }

    private fun detectAttachment(vararg texts: String): Boolean {
        val attachmentIndicators = listOf("attachment", "attached", "📎", "📄")
        return texts.any { text ->
            attachmentIndicators.any { text.lowercase().contains(it) }
        }
    }
}
