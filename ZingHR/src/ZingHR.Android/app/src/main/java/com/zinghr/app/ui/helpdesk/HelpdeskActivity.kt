package com.zinghr.app.ui.helpdesk

import android.content.Intent
import android.os.Bundle
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import com.zinghr.app.adapters.TicketAdapter
import com.zinghr.app.api.RetrofitClient
import com.zinghr.app.databinding.ActivityHelpdeskBinding
import kotlinx.coroutines.launch

class HelpdeskActivity : AppCompatActivity() {

    private lateinit var binding: ActivityHelpdeskBinding

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityHelpdeskBinding.inflate(layoutInflater)
        setContentView(binding.root)

        binding.toolbar.setNavigationOnClickListener { finish() }
        binding.rvTickets.layoutManager = LinearLayoutManager(this)
        binding.fabCreateTicket.setOnClickListener {
            startActivity(Intent(this, CreateTicketActivity::class.java))
        }
    }

    override fun onResume() {
        super.onResume()
        loadTickets()
    }

    private fun loadTickets() {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@HelpdeskActivity)
                val response = api.getMyTickets()
                if (response.isSuccessful && response.body() != null) {
                    val list = response.body()!!
                    if (list.isEmpty()) {
                        binding.tvEmpty.visibility = View.VISIBLE
                        binding.rvTickets.visibility = View.GONE
                    } else {
                        binding.tvEmpty.visibility = View.GONE
                        binding.rvTickets.visibility = View.VISIBLE
                        binding.rvTickets.adapter = TicketAdapter(list)
                    }
                }
            } catch (e: Exception) {
                Toast.makeText(this@HelpdeskActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            }
        }
    }
}
