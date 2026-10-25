package com.voiceai.mailtrack.data.entity

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "dlp_rules")
data class DlpRule(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val name: String,
    val description: String,
    val ruleType: String, // "keyword", "domain", "regex", "attachment_type", "recipient_count"
    val pattern: String, // the keyword, domain, regex pattern, etc.
    val direction: String, // "incoming", "outgoing", "both"
    val severity: String, // "low", "medium", "high", "critical"
    val action: String, // "alert", "flag", "block"
    val enabled: Boolean = true,
    val createdAt: Long = System.currentTimeMillis()
)
