package com.zinghr.app.ui.expense

import android.app.DatePickerDialog
import android.os.Bundle
import android.view.View
import android.widget.ArrayAdapter
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.zinghr.app.api.ExpenseCategoryDto
import com.zinghr.app.api.ExpenseSubmitRequest
import com.zinghr.app.api.RetrofitClient
import com.zinghr.app.databinding.ActivitySubmitExpenseBinding
import kotlinx.coroutines.launch
import java.util.Calendar

class SubmitExpenseActivity : AppCompatActivity() {

    private lateinit var binding: ActivitySubmitExpenseBinding
    private var categories = listOf<ExpenseCategoryDto>()
    private var expenseDate = ""

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivitySubmitExpenseBinding.inflate(layoutInflater)
        setContentView(binding.root)

        binding.toolbar.setNavigationOnClickListener { finish() }
        binding.etDate.setOnClickListener { pickDate() }
        binding.btnSubmit.setOnClickListener { submit() }
        loadCategories()
    }

    private fun loadCategories() {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@SubmitExpenseActivity)
                val response = api.getExpenseCategories()
                if (response.isSuccessful && response.body() != null) {
                    categories = response.body()!!
                    val names = categories.map { it.name }
                    binding.spinnerCategory.adapter = ArrayAdapter(this@SubmitExpenseActivity, android.R.layout.simple_spinner_dropdown_item, names)
                }
            } catch (_: Exception) { }
        }
    }

    private fun pickDate() {
        val cal = Calendar.getInstance()
        DatePickerDialog(this, { _, y, m, d ->
            expenseDate = String.format("%04d-%02d-%02d", y, m + 1, d)
            binding.etDate.setText(expenseDate)
        }, cal.get(Calendar.YEAR), cal.get(Calendar.MONTH), cal.get(Calendar.DAY_OF_MONTH)).show()
    }

    private fun submit() {
        if (categories.isEmpty()) return
        val title = binding.etTitle.text.toString().trim()
        val amountStr = binding.etAmount.text.toString().trim()

        if (title.isEmpty() || amountStr.isEmpty() || expenseDate.isEmpty()) {
            Toast.makeText(this, "Fill all required fields", Toast.LENGTH_SHORT).show()
            return
        }

        binding.progressBar.visibility = View.VISIBLE
        binding.btnSubmit.isEnabled = false

        val req = ExpenseSubmitRequest(
            categoryId = categories[binding.spinnerCategory.selectedItemPosition].id,
            title = title,
            amount = amountStr.toDouble(),
            expenseDate = expenseDate,
            description = binding.etDescription.text.toString().takeIf { it.isNotBlank() },
            receiptUrl = null
        )

        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@SubmitExpenseActivity)
                val response = api.submitExpense(req)
                if (response.isSuccessful) {
                    Toast.makeText(this@SubmitExpenseActivity, "Expense submitted!", Toast.LENGTH_SHORT).show()
                    finish()
                } else {
                    Toast.makeText(this@SubmitExpenseActivity, response.errorBody()?.string() ?: "Failed", Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                Toast.makeText(this@SubmitExpenseActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            } finally {
                binding.progressBar.visibility = View.GONE
                binding.btnSubmit.isEnabled = true
            }
        }
    }
}
