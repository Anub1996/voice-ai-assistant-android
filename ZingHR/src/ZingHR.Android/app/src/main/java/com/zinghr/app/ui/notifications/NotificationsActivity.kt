package com.zinghr.app.ui.notifications

import android.os.Bundle
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import com.zinghr.app.adapters.NotificationAdapter
import com.zinghr.app.api.RetrofitClient
import com.zinghr.app.databinding.ActivityNotificationsBinding
import kotlinx.coroutines.launch

class NotificationsActivity : AppCompatActivity() {

    private lateinit var binding: ActivityNotificationsBinding

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityNotificationsBinding.inflate(layoutInflater)
        setContentView(binding.root)

        binding.toolbar.setNavigationOnClickListener { finish() }
        binding.rvNotifications.layoutManager = LinearLayoutManager(this)
        binding.btnMarkAllRead.setOnClickListener { markAllRead() }

        loadNotifications()
    }

    private fun loadNotifications() {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@NotificationsActivity)
                val response = api.getNotifications()
                if (response.isSuccessful && response.body() != null) {
                    val page = response.body()!!
                    if (page.data.isEmpty()) {
                        binding.tvEmpty.visibility = View.VISIBLE
                        binding.rvNotifications.visibility = View.GONE
                    } else {
                        binding.tvEmpty.visibility = View.GONE
                        binding.rvNotifications.visibility = View.VISIBLE
                        binding.rvNotifications.adapter = NotificationAdapter(page.data) { id ->
                            markRead(id)
                        }
                    }
                }
            } catch (e: Exception) {
                Toast.makeText(this@NotificationsActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun markRead(id: Long) {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@NotificationsActivity)
                api.markNotificationRead(id)
                loadNotifications()
            } catch (_: Exception) { }
        }
    }

    private fun markAllRead() {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@NotificationsActivity)
                api.markAllNotificationsRead()
                Toast.makeText(this@NotificationsActivity, "All marked as read", Toast.LENGTH_SHORT).show()
                loadNotifications()
            } catch (e: Exception) {
                Toast.makeText(this@NotificationsActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            }
        }
    }
}
