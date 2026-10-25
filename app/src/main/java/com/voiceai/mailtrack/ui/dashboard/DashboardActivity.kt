package com.voiceai.mailtrack.ui.dashboard

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.provider.Settings
import android.text.TextUtils
import android.view.Menu
import android.view.MenuItem
import androidx.activity.viewModels
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.fragment.app.Fragment
import com.google.android.material.bottomnavigation.BottomNavigationView
import com.google.android.material.snackbar.Snackbar
import com.voiceai.mailtrack.R
import com.voiceai.mailtrack.service.MailTrackForegroundService
import com.voiceai.mailtrack.ui.analytics.AnalyticsFragment
import com.voiceai.mailtrack.ui.emaillist.EmailListFragment
import com.voiceai.mailtrack.ui.ruleconfig.RuleConfigActivity

class DashboardActivity : AppCompatActivity() {

    private val viewModel: DashboardViewModel by viewModels()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_dashboard)
        setSupportActionBar(findViewById(R.id.toolbar))
        supportActionBar?.title = "Mail Track"

        setupBottomNavigation()
        checkNotificationListenerPermission()
        startMonitoringService()

        if (savedInstanceState == null) {
            val startTab = intent.getStringExtra("tab")
            if (startTab == "alerts") {
                loadFragment(EmailListFragment.newInstance("alerts"))
            } else {
                loadFragment(EmailListFragment.newInstance("all"))
            }
        }
    }

    private fun setupBottomNavigation() {
        findViewById<BottomNavigationView>(R.id.bottom_navigation).setOnItemSelectedListener { item ->
            when (item.itemId) {
                R.id.nav_all -> {
                    loadFragment(EmailListFragment.newInstance("all"))
                    true
                }
                R.id.nav_incoming -> {
                    loadFragment(EmailListFragment.newInstance("incoming"))
                    true
                }
                R.id.nav_outgoing -> {
                    loadFragment(EmailListFragment.newInstance("outgoing"))
                    true
                }
                R.id.nav_alerts -> {
                    loadFragment(EmailListFragment.newInstance("alerts"))
                    true
                }
                R.id.nav_analytics -> {
                    loadFragment(AnalyticsFragment())
                    true
                }
                else -> false
            }
        }
    }

    private fun loadFragment(fragment: Fragment) {
        supportFragmentManager.beginTransaction()
            .replace(R.id.fragment_container, fragment)
            .commit()
    }

    override fun onCreateOptionsMenu(menu: Menu): Boolean {
        menuInflater.inflate(R.menu.menu_dashboard, menu)
        return true
    }

    override fun onOptionsItemSelected(item: MenuItem): Boolean {
        return when (item.itemId) {
            R.id.action_rules -> {
                startActivity(Intent(this, RuleConfigActivity::class.java))
                true
            }
            R.id.action_clear_alerts -> {
                viewModel.acknowledgeAllAlerts()
                Snackbar.make(
                    findViewById(R.id.fragment_container),
                    "All alerts acknowledged",
                    Snackbar.LENGTH_SHORT
                ).show()
                true
            }
            R.id.action_settings -> {
                showSettingsDialog()
                true
            }
            else -> super.onOptionsItemSelected(item)
        }
    }

    private fun showSettingsDialog() {
        val prefs = getSharedPreferences("mail_track_prefs", Context.MODE_PRIVATE)
        val isEnabled = prefs.getBoolean("monitoring_enabled", true)

        AlertDialog.Builder(this)
            .setTitle("Monitoring Settings")
            .setMessage("Email monitoring is currently ${if (isEnabled) "ENABLED" else "DISABLED"}")
            .setPositiveButton(if (isEnabled) "Disable" else "Enable") { _, _ ->
                prefs.edit().putBoolean("monitoring_enabled", !isEnabled).apply()
                if (isEnabled) {
                    stopService(Intent(this, MailTrackForegroundService::class.java))
                } else {
                    startMonitoringService()
                }
            }
            .setNeutralButton("Notification Access") { _, _ ->
                startActivity(Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS))
            }
            .setNegativeButton("Cancel", null)
            .show()
    }

    private fun checkNotificationListenerPermission() {
        if (!isNotificationListenerEnabled()) {
            AlertDialog.Builder(this)
                .setTitle("Permission Required")
                .setMessage("Mail Track needs Notification Access to monitor email notifications. Please enable it in settings.")
                .setPositiveButton("Open Settings") { _, _ ->
                    startActivity(Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS))
                }
                .setNegativeButton("Later", null)
                .setCancelable(false)
                .show()
        }
    }

    private fun isNotificationListenerEnabled(): Boolean {
        val flat = Settings.Secure.getString(contentResolver, "enabled_notification_listeners")
        if (TextUtils.isEmpty(flat)) return false
        val names = flat.split(":")
        val componentName = ComponentName(this, "com.voiceai.mailtrack.service.EmailNotificationListenerService")
        return names.any { ComponentName.unflattenFromString(it) == componentName }
    }

    private fun startMonitoringService() {
        val prefs = getSharedPreferences("mail_track_prefs", Context.MODE_PRIVATE)
        if (prefs.getBoolean("monitoring_enabled", true)) {
            ContextCompat.startForegroundService(
                this, Intent(this, MailTrackForegroundService::class.java)
            )
        }
    }
}
