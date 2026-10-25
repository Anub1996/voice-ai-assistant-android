package com.voiceai.mailtrack.data.entity

import androidx.room.Entity
import androidx.room.PrimaryKey

@Entity(tableName = "alerts")
data class Alert(
    @PrimaryKey(autoGenerate = true) val id: Long = 0,
    val emailId: Long,
    val ruleId: Long,
    val ruleName: String,
    val severity: String,
    val description: String,
    val acknowledged: Boolean = false,
    val timestamp: Long = System.currentTimeMillis()
)
