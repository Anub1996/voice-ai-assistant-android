package com.voiceai.mailtrack.ui.emaildetail

import android.os.Bundle
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.voiceai.mailtrack.MailTrackApp
import com.voiceai.mailtrack.R
import com.voiceai.mailtrack.data.repository.MailTrackRepository
import kotlinx.coroutines.launch
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

class EmailDetailActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_email_detail)
        setSupportActionBar(findViewById(R.id.toolbar))
        supportActionBar?.setDisplayHomeAsUpEnabled(true)
        supportActionBar?.title = "Email Details"

        val emailId = intent.getLongExtra("email_id", -1)
        if (emailId == -1L) {
            finish()
            return
        }

        val db = MailTrackApp.instance.database
        val repository = MailTrackRepository(db.emailDao(), db.ruleDao(), db.alertDao())

        lifecycleScope.launch {
            val email = repository.getEmailById(emailId) ?: run {
                finish()
                return@launch
            }

            val dateFormat = SimpleDateFormat("EEEE, MMM d yyyy 'at' HH:mm:ss", Locale.getDefault())

            findViewById<TextView>(R.id.tv_detail_sender).text = email.sender.ifBlank { "(unknown)" }
            findViewById<TextView>(R.id.tv_detail_recipients).text = email.recipients.ifBlank { "(not captured)" }
            findViewById<TextView>(R.id.tv_detail_subject).text = email.subject.ifBlank { "(no subject)" }
            findViewById<TextView>(R.id.tv_detail_body).text = email.bodyPreview.ifBlank { "(no content captured)" }
            findViewById<TextView>(R.id.tv_detail_timestamp).text = dateFormat.format(Date(email.timestamp))
            findViewById<TextView>(R.id.tv_detail_direction).text = email.direction.replaceFirstChar { it.uppercase() }
            findViewById<TextView>(R.id.tv_detail_client).text = email.emailClient
            findViewById<TextView>(R.id.tv_detail_attachment).text =
                if (email.hasAttachment) email.attachmentNames ?: "Yes" else "None"

            if (email.alertTriggered) {
                findViewById<TextView>(R.id.tv_detail_alert_status).text = "ALERT TRIGGERED"
                val alerts = repository.getAlertsByEmailId(emailId)
                if (alerts.isNotEmpty()) {
                    val alertText = alerts.joinToString("\n") { "- ${it.ruleName}: ${it.description}" }
                    findViewById<TextView>(R.id.tv_detail_alert_details).text = alertText
                }
            } else {
                findViewById<TextView>(R.id.tv_detail_alert_status).text = "Clean"
                findViewById<TextView>(R.id.tv_detail_alert_details).text = ""
            }
        }
    }

    override fun onSupportNavigateUp(): Boolean {
        onBackPressedDispatcher.onBackPressed()
        return true
    }
}
