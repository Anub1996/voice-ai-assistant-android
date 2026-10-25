package com.voiceai.mailtrack

import android.app.Application
import android.app.NotificationChannel
import android.app.NotificationManager
import com.voiceai.mailtrack.data.db.MailTrackDatabase

class MailTrackApp : Application() {

    val database: MailTrackDatabase by lazy { MailTrackDatabase.getInstance(this) }

    override fun onCreate() {
        super.onCreate()
        instance = this
        createNotificationChannels()
    }

    private fun createNotificationChannels() {
        val manager = getSystemService(NotificationManager::class.java)

        val serviceChannel = NotificationChannel(
            CHANNEL_SERVICE, "Mail Tracking Service", NotificationManager.IMPORTANCE_LOW
        ).apply { description = "Background email monitoring" }

        val alertChannel = NotificationChannel(
            CHANNEL_ALERTS, "Mail Track Alerts", NotificationManager.IMPORTANCE_HIGH
        ).apply { description = "DLP policy violation alerts" }

        manager.createNotificationChannel(serviceChannel)
        manager.createNotificationChannel(alertChannel)
    }

    companion object {
        const val CHANNEL_SERVICE = "mail_track_service"
        const val CHANNEL_ALERTS = "mail_track_alerts"

        lateinit var instance: MailTrackApp
            private set
    }
}
