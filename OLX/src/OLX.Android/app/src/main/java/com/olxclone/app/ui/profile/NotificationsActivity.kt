package com.olxclone.app.ui.profile

import android.os.Bundle
import android.view.View
import android.widget.ImageButton
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.olxclone.app.R
import com.olxclone.app.adapters.NotificationAdapter
import com.olxclone.app.api.RetrofitClient
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class NotificationsActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_notifications)

        val api = RetrofitClient.getInstance(this)
        val btnBack = findViewById<ImageButton>(R.id.btnBack)
        val tvMarkAll = findViewById<TextView>(R.id.tvMarkAll)
        val tvEmpty = findViewById<TextView>(R.id.tvEmpty)
        val rvNotifications = findViewById<RecyclerView>(R.id.rvNotifications)

        rvNotifications.layoutManager = LinearLayoutManager(this)
        btnBack.setOnClickListener { finish() }

        fun load() {
            lifecycleScope.launch {
                try {
                    val resp = withContext(Dispatchers.IO) { api.getNotifications() }
                    if (resp.isSuccessful) {
                        val items = resp.body() ?: emptyList()
                        if (items.isEmpty()) {
                            tvEmpty.visibility = View.VISIBLE
                        } else {
                            tvEmpty.visibility = View.GONE
                            val adapter = NotificationAdapter { notif ->
                                lifecycleScope.launch {
                                    try {
                                        withContext(Dispatchers.IO) { api.markRead(notif.id) }
                                    } catch (_: Exception) {}
                                }
                            }
                            adapter.updateData(items)
                            rvNotifications.adapter = adapter
                        }
                    }
                } catch (e: Exception) {
                    Toast.makeText(this@NotificationsActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
                }
            }
        }

        load()

        tvMarkAll.setOnClickListener {
            lifecycleScope.launch {
                try {
                    withContext(Dispatchers.IO) { api.markAllRead() }
                    Toast.makeText(this@NotificationsActivity, "All marked as read", Toast.LENGTH_SHORT).show()
                    load()
                } catch (_: Exception) {}
            }
        }
    }
}
