package com.voiceai.mailtrack.service

import android.app.Notification
import android.app.PendingIntent
import android.content.Intent
import android.service.notification.NotificationListenerService
import android.service.notification.StatusBarNotification
import androidx.core.app.NotificationCompat
import com.voiceai.mailtrack.MailTrackApp
import com.voiceai.mailtrack.R
import com.voiceai.mailtrack.data.entity.Alert
import com.voiceai.mailtrack.data.entity.TrackedEmail
import com.voiceai.mailtrack.data.repository.MailTrackRepository
import com.voiceai.mailtrack.domain.rules.RuleEngine
import com.voiceai.mailtrack.ui.dashboard.DashboardActivity
import com.voiceai.mailtrack.util.NotificationParser
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch

class EmailNotificationListenerService : NotificationListenerService() {

    private val job = SupervisorJob()
    private val scope = CoroutineScope(Dispatchers.IO + job)
    private val ruleEngine = RuleEngine()
    private lateinit var repository: MailTrackRepository

    override fun onCreate() {
        super.onCreate()
        val db = MailTrackApp.instance.database
        repository = MailTrackRepository(db.emailDao(), db.ruleDao(), db.alertDao())
    }

    override fun onNotificationPosted(sbn: StatusBarNotification) {
        val parsed = NotificationParser.parse(sbn) ?: return

        scope.launch {
            val email = TrackedEmail(
                sender = parsed.sender,
                recipients = parsed.recipients,
                subject = parsed.subject,
                bodyPreview = parsed.bodyPreview,
                direction = parsed.direction,
                emailClient = parsed.emailClient,
                hasAttachment = parsed.hasAttachment
            )

            val emailId = repository.insertEmail(email)
            val savedEmail = email.copy(id = emailId)

            val rules = repository.getEnabledRules()
            val violations = ruleEngine.evaluate(savedEmail, rules)

            if (violations.isNotEmpty()) {
                repository.updateEmail(savedEmail.copy(
                    alertTriggered = true,
                    alertRuleId = violations.first().rule.id
                ))

                for (violation in violations) {
                    val alert = Alert(
                        emailId = emailId,
                        ruleId = violation.rule.id,
                        ruleName = violation.rule.name,
                        severity = violation.rule.severity,
                        description = violation.matchedText
                    )
                    repository.insertAlert(alert)
                }

                showAlertNotification(savedEmail, violations.first().rule.name)
            }
        }
    }

    override fun onNotificationRemoved(sbn: StatusBarNotification) {}

    private fun showAlertNotification(email: TrackedEmail, ruleName: String) {
        val intent = Intent(this, DashboardActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP
            putExtra("tab", "alerts")
        }
        val pendingIntent = PendingIntent.getActivity(
            this, 0, intent, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val notification = NotificationCompat.Builder(this, MailTrackApp.CHANNEL_ALERTS)
            .setSmallIcon(R.drawable.ic_alert)
            .setContentTitle("Mail Track Alert: $ruleName")
            .setContentText("${email.direction.replaceFirstChar { it.uppercase() }} email from ${email.sender}")
            .setStyle(NotificationCompat.BigTextStyle()
                .bigText("Rule violated: $ruleName\nFrom: ${email.sender}\nSubject: ${email.subject}"))
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setContentIntent(pendingIntent)
            .setAutoCancel(true)
            .build()

        val manager = getSystemService(NOTIFICATION_SERVICE) as android.app.NotificationManager
        manager.notify(email.id.toInt(), notification)
    }

    override fun onDestroy() {
        super.onDestroy()
        job.cancel()
    }
}
