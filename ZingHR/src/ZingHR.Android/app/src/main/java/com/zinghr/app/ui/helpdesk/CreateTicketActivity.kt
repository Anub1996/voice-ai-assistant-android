package com.zinghr.app.ui.helpdesk

import android.os.Bundle
import android.view.View
import android.widget.ArrayAdapter
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.zinghr.app.api.CreateTicketRequest
import com.zinghr.app.api.RetrofitClient
import com.zinghr.app.databinding.ActivityCreateTicketBinding
import kotlinx.coroutines.launch

class CreateTicketActivity : AppCompatActivity() {

    private lateinit var binding: ActivityCreateTicketBinding

    private val categories = listOf("it", "hr", "payroll", "admin", "general")
    private val priorities = listOf("low", "medium", "high", "urgent")

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityCreateTicketBinding.inflate(layoutInflater)
        setContentView(binding.root)

        binding.toolbar.setNavigationOnClickListener { finish() }

        binding.spinnerCategory.adapter = ArrayAdapter(this, android.R.layout.simple_spinner_dropdown_item,
            categories.map { it.replaceFirstChar { c -> c.uppercase() } })
        binding.spinnerPriority.adapter = ArrayAdapter(this, android.R.layout.simple_spinner_dropdown_item,
            priorities.map { it.replaceFirstChar { c -> c.uppercase() } })
        binding.spinnerPriority.setSelection(1)

        binding.btnSubmit.setOnClickListener { submit() }
    }

    private fun submit() {
        val subject = binding.etSubject.text.toString().trim()
        val description = binding.etDescription.text.toString().trim()

        if (subject.isEmpty() || description.isEmpty()) {
            Toast.makeText(this, "Fill all required fields", Toast.LENGTH_SHORT).show()
            return
        }

        binding.progressBar.visibility = View.VISIBLE
        binding.btnSubmit.isEnabled = false

        val req = CreateTicketRequest(
            category = categories[binding.spinnerCategory.selectedItemPosition],
            subject = subject,
            description = description,
            priority = priorities[binding.spinnerPriority.selectedItemPosition]
        )

        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@CreateTicketActivity)
                val response = api.createTicket(req)
                if (response.isSuccessful) {
                    Toast.makeText(this@CreateTicketActivity, "Ticket created!", Toast.LENGTH_SHORT).show()
                    finish()
                } else {
                    Toast.makeText(this@CreateTicketActivity, response.errorBody()?.string() ?: "Failed", Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                Toast.makeText(this@CreateTicketActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            } finally {
                binding.progressBar.visibility = View.GONE
                binding.btnSubmit.isEnabled = true
            }
        }
    }
}
