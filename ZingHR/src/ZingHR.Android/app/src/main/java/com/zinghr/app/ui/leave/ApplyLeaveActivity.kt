package com.zinghr.app.ui.leave

import android.app.DatePickerDialog
import android.os.Bundle
import android.view.View
import android.widget.ArrayAdapter
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.zinghr.app.api.LeaveApplyRequest
import com.zinghr.app.api.LeaveTypeDto
import com.zinghr.app.api.RetrofitClient
import com.zinghr.app.databinding.ActivityApplyLeaveBinding
import kotlinx.coroutines.launch
import java.util.Calendar

class ApplyLeaveActivity : AppCompatActivity() {

    private lateinit var binding: ActivityApplyLeaveBinding
    private var leaveTypes = listOf<LeaveTypeDto>()
    private var fromDate: String = ""
    private var toDate: String = ""

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityApplyLeaveBinding.inflate(layoutInflater)
        setContentView(binding.root)

        binding.toolbar.setNavigationOnClickListener { finish() }

        binding.etFromDate.setOnClickListener { pickDate { fromDate = it; binding.etFromDate.setText(it) } }
        binding.etToDate.setOnClickListener { pickDate { toDate = it; binding.etToDate.setText(it) } }

        binding.btnSubmit.setOnClickListener { submitLeave() }
        loadLeaveTypes()
    }

    private fun loadLeaveTypes() {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@ApplyLeaveActivity)
                val response = api.getLeaveTypes()
                if (response.isSuccessful && response.body() != null) {
                    leaveTypes = response.body()!!
                    val names = leaveTypes.map { "${it.name} (${it.code})" }
                    binding.spinnerLeaveType.adapter = ArrayAdapter(this@ApplyLeaveActivity, android.R.layout.simple_spinner_dropdown_item, names)
                }
            } catch (_: Exception) { }
        }
    }

    private fun pickDate(onPicked: (String) -> Unit) {
        val cal = Calendar.getInstance()
        DatePickerDialog(this, { _, y, m, d ->
            onPicked(String.format("%04d-%02d-%02d", y, m + 1, d))
        }, cal.get(Calendar.YEAR), cal.get(Calendar.MONTH), cal.get(Calendar.DAY_OF_MONTH)).show()
    }

    private fun submitLeave() {
        if (leaveTypes.isEmpty()) return
        val selectedIndex = binding.spinnerLeaveType.selectedItemPosition
        val reason = binding.etReason.text.toString().trim()

        if (fromDate.isEmpty() || toDate.isEmpty() || reason.isEmpty()) {
            Toast.makeText(this, "Fill all required fields", Toast.LENGTH_SHORT).show()
            return
        }

        binding.progressBar.visibility = View.VISIBLE
        binding.btnSubmit.isEnabled = false

        val req = LeaveApplyRequest(
            leaveTypeId = leaveTypes[selectedIndex].id,
            fromDate = fromDate,
            toDate = toDate,
            isHalfDay = binding.cbHalfDay.isChecked,
            halfDayType = if (binding.cbHalfDay.isChecked) "first-half" else null,
            reason = reason,
            emergencyContact = binding.etEmergencyContact.text.toString().takeIf { it.isNotBlank() }
        )

        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@ApplyLeaveActivity)
                val response = api.applyLeave(req)
                if (response.isSuccessful) {
                    Toast.makeText(this@ApplyLeaveActivity, "Leave applied!", Toast.LENGTH_SHORT).show()
                    finish()
                } else {
                    val error = response.errorBody()?.string() ?: "Failed"
                    Toast.makeText(this@ApplyLeaveActivity, error, Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                Toast.makeText(this@ApplyLeaveActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            } finally {
                binding.progressBar.visibility = View.GONE
                binding.btnSubmit.isEnabled = true
            }
        }
    }
}
