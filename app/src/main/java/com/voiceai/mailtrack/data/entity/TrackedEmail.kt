package com.voiceai.mailtrack.data.entity

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "tracked_emails")
data class TrackedEmail(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val sender: String,
    val recipients: String,
    val subject: String,
    val bodyPreview: String,
    val direction: String, // "incoming" or "outgoing"
    val emailClient: String, // "gmail", "outlook", "yahoo", etc.
    val hasAttachment: Boolean = false,
    val attachmentNames: String? = null,
    val timestamp: Long = System.currentTimeMillis(),
    val flagged: Boolean = false,
    val alertTriggered: Boolean = false,
    val alertRuleId: Long? = null
)
