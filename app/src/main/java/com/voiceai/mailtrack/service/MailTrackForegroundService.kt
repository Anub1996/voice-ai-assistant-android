package com.voiceai.mailtrack.service

import android.app.PendingIntent
import android.app.Service
import android.content.Intent
import android.os.IBinder
import androidx.core.app.NotificationCompat
import com.voiceai.mailtrack.MailTrackApp
import com.voiceai.mailtrack.R
import com.voiceai.mailtrack.ui.dashboard.DashboardActivity

class MailTrackForegroundService : Service() {

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val tapIntent = Intent(this, DashboardActivity::class.java)
        val pendingIntent = PendingIntent.getActivity(
            this, 0, tapIntent, PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val notification = NotificationCompat.Builder(this, MailTrackApp.CHANNEL_SERVICE)
            .setContentTitle("Mail Track Active")
            .setContentText("Monitoring email activity")
            .setSmallIcon(R.drawable.ic_mail_track)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .build()

        startForeground(NOTIFICATION_ID, notification)
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    companion object {
        private const val NOTIFICATION_ID = 1001
    }
}
