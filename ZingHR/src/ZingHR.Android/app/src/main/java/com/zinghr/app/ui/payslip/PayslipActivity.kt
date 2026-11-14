package com.zinghr.app.ui.payslip

import android.content.Intent
import android.os.Bundle
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import com.zinghr.app.adapters.PayslipAdapter
import com.zinghr.app.api.RetrofitClient
import com.zinghr.app.databinding.ActivityPayslipBinding
import kotlinx.coroutines.launch

class PayslipActivity : AppCompatActivity() {

    private lateinit var binding: ActivityPayslipBinding

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityPayslipBinding.inflate(layoutInflater)
        setContentView(binding.root)

        binding.toolbar.setNavigationOnClickListener { finish() }
        binding.rvPayslips.layoutManager = LinearLayoutManager(this)
        loadPayslips()
    }

    private fun loadPayslips() {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@PayslipActivity)
                val response = api.getPayslips()
                if (response.isSuccessful && response.body() != null) {
                    val list = response.body()!!
                    if (list.isEmpty()) {
                        binding.tvEmpty.visibility = View.VISIBLE
                        binding.rvPayslips.visibility = View.GONE
                    } else {
                        binding.tvEmpty.visibility = View.GONE
                        binding.rvPayslips.visibility = View.VISIBLE
                        binding.rvPayslips.adapter = PayslipAdapter(list) { id ->
                            val intent = Intent(this@PayslipActivity, PayslipDetailActivity::class.java)
                            intent.putExtra("payslipId", id)
                            startActivity(intent)
                        }
                    }
                }
            } catch (e: Exception) {
                Toast.makeText(this@PayslipActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            }
        }
    }
}
