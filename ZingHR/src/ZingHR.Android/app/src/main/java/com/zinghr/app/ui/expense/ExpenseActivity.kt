package com.zinghr.app.ui.expense

import android.content.Intent
import android.os.Bundle
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import com.zinghr.app.adapters.ExpenseAdapter
import com.zinghr.app.api.RetrofitClient
import com.zinghr.app.databinding.ActivityExpenseBinding
import kotlinx.coroutines.launch

class ExpenseActivity : AppCompatActivity() {

    private lateinit var binding: ActivityExpenseBinding

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityExpenseBinding.inflate(layoutInflater)
        setContentView(binding.root)

        binding.toolbar.setNavigationOnClickListener { finish() }
        binding.rvExpenses.layoutManager = LinearLayoutManager(this)
        binding.fabSubmitExpense.setOnClickListener {
            startActivity(Intent(this, SubmitExpenseActivity::class.java))
        }
    }

    override fun onResume() {
        super.onResume()
        loadExpenses()
    }

    private fun loadExpenses() {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@ExpenseActivity)
                val response = api.getExpenses()
                if (response.isSuccessful && response.body() != null) {
                    val list = response.body()!!
                    if (list.isEmpty()) {
                        binding.tvEmpty.visibility = View.VISIBLE
                        binding.rvExpenses.visibility = View.GONE
                    } else {
                        binding.tvEmpty.visibility = View.GONE
                        binding.rvExpenses.visibility = View.VISIBLE
                        binding.rvExpenses.adapter = ExpenseAdapter(list)
                    }
                }
            } catch (e: Exception) {
                Toast.makeText(this@ExpenseActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            }
        }
    }
}
