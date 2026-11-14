package com.zinghr.app.ui.leave

import android.content.Intent
import android.os.Bundle
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import com.zinghr.app.adapters.LeaveBalanceAdapter
import com.zinghr.app.adapters.LeaveRequestAdapter
import com.zinghr.app.api.RetrofitClient
import com.zinghr.app.databinding.ActivityLeaveBinding
import kotlinx.coroutines.launch

class LeaveActivity : AppCompatActivity() {

    private lateinit var binding: ActivityLeaveBinding

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityLeaveBinding.inflate(layoutInflater)
        setContentView(binding.root)

        binding.toolbar.setNavigationOnClickListener { finish() }
        binding.fabApplyLeave.setOnClickListener {
            startActivity(Intent(this, ApplyLeaveActivity::class.java))
        }

        binding.rvBalances.layoutManager = LinearLayoutManager(this, LinearLayoutManager.HORIZONTAL, false)
        binding.rvHistory.layoutManager = LinearLayoutManager(this)

        loadData()
    }

    override fun onResume() {
        super.onResume()
        loadData()
    }

    private fun loadData() {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@LeaveActivity)

                val balResponse = api.getLeaveBalance()
                if (balResponse.isSuccessful && balResponse.body() != null) {
                    binding.rvBalances.adapter = LeaveBalanceAdapter(balResponse.body()!!)
                }

                val histResponse = api.getLeaveHistory()
                if (histResponse.isSuccessful && histResponse.body() != null) {
                    val requests = histResponse.body()!!
                    if (requests.isEmpty()) {
                        binding.tvEmpty.visibility = View.VISIBLE
                        binding.rvHistory.visibility = View.GONE
                    } else {
                        binding.tvEmpty.visibility = View.GONE
                        binding.rvHistory.visibility = View.VISIBLE
                        binding.rvHistory.adapter = LeaveRequestAdapter(requests) { id ->
                            cancelLeave(id)
                        }
                    }
                }
            } catch (e: Exception) {
                Toast.makeText(this@LeaveActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun cancelLeave(id: Long) {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@LeaveActivity)
                val response = api.cancelLeave(id)
                if (response.isSuccessful) {
                    Toast.makeText(this@LeaveActivity, "Leave cancelled", Toast.LENGTH_SHORT).show()
                    loadData()
                }
            } catch (e: Exception) {
                Toast.makeText(this@LeaveActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            }
        }
    }
}
