package com.voiceai.mailtrack.service

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import androidx.core.content.ContextCompat

class BootReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action == Intent.ACTION_BOOT_COMPLETED) {
            val prefs = context.getSharedPreferences("mail_track_prefs", Context.MODE_PRIVATE)
            if (prefs.getBoolean("monitoring_enabled", false)) {
                val serviceIntent = Intent(context, MailTrackForegroundService::class.java)
                ContextCompat.startForegroundService(context, serviceIntent)
            }
        }
    }
}
