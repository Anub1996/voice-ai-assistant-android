package com.zinghr.app.ui.profile

import android.os.Bundle
import android.widget.Toast
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.zinghr.app.api.ProfileUpdateRequest
import com.zinghr.app.api.RetrofitClient
import com.zinghr.app.databinding.ActivityProfileBinding
import kotlinx.coroutines.launch

class ProfileActivity : AppCompatActivity() {

    private lateinit var binding: ActivityProfileBinding

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityProfileBinding.inflate(layoutInflater)
        setContentView(binding.root)

        binding.toolbar.setNavigationOnClickListener { finish() }
        binding.btnEditPhone.setOnClickListener { showEditPhoneDialog() }
        loadProfile()
    }

    private fun loadProfile() {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@ProfileActivity)
                val response = api.getProfile()
                if (response.isSuccessful && response.body() != null) {
                    val p = response.body()!!
                    binding.tvName.text = p.fullName
                    binding.tvEmpCode.text = p.empCode
                    binding.tvEmail.text = p.email
                    binding.tvPhone.text = p.phone.ifEmpty { "Not set" }
                    binding.tvGender.text = p.gender.ifEmpty { "Not set" }
                    binding.tvDepartment.text = p.department
                    binding.tvDesignation.text = p.designation
                    binding.tvManager.text = p.reportingManager.ifEmpty { "N/A" }
                    binding.tvDoj.text = p.dateOfJoining
                    binding.tvEmpType.text = p.employmentType.replaceFirstChar { it.uppercase() }
                    binding.tvStatus.text = p.status.replaceFirstChar { it.uppercase() }
                }
            } catch (e: Exception) {
                Toast.makeText(this@ProfileActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun showEditPhoneDialog() {
        val input = android.widget.EditText(this)
        input.hint = "Enter phone number"
        input.inputType = android.text.InputType.TYPE_CLASS_PHONE

        AlertDialog.Builder(this)
            .setTitle("Update Phone")
            .setView(input)
            .setPositiveButton("Save") { _, _ ->
                val phone = input.text.toString().trim()
                if (phone.isNotEmpty()) updatePhone(phone)
            }
            .setNegativeButton("Cancel", null)
            .show()
    }

    private fun updatePhone(phone: String) {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@ProfileActivity)
                val response = api.updateProfile(ProfileUpdateRequest(phone = phone, address = null, city = null, state = null, pinCode = null, emergencyContact = null))
                if (response.isSuccessful) {
                    Toast.makeText(this@ProfileActivity, "Phone updated", Toast.LENGTH_SHORT).show()
                    binding.tvPhone.text = phone
                }
            } catch (e: Exception) {
                Toast.makeText(this@ProfileActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            }
        }
    }
}
