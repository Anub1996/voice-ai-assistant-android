package com.olxclone.app.ui.auth

import android.content.Intent
import android.os.Bundle
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.olxclone.app.R
import com.olxclone.app.api.RegisterRequest
import com.olxclone.app.api.RetrofitClient
import com.olxclone.app.ui.home.MainActivity
import com.olxclone.app.utils.SessionManager
import android.widget.Button
import android.widget.EditText
import android.widget.ProgressBar
import android.widget.TextView
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class RegisterActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_register)

        val etFullName = findViewById<EditText>(R.id.etFullName)
        val etUsername = findViewById<EditText>(R.id.etUsername)
        val etEmail = findViewById<EditText>(R.id.etEmail)
        val etPhone = findViewById<EditText>(R.id.etPhone)
        val etPassword = findViewById<EditText>(R.id.etPassword)
        val btnRegister = findViewById<Button>(R.id.btnRegister)
        val tvLogin = findViewById<TextView>(R.id.tvLogin)
        val progressBar = findViewById<ProgressBar>(R.id.progressBar)

        btnRegister.setOnClickListener {
            val fullName = etFullName.text.toString().trim()
            val username = etUsername.text.toString().trim()
            val email = etEmail.text.toString().trim()
            val phone = etPhone.text.toString().trim()
            val password = etPassword.text.toString().trim()

            if (fullName.isEmpty() || username.isEmpty() || email.isEmpty() || phone.isEmpty() || password.isEmpty()) {
                Toast.makeText(this, "Please fill all fields", Toast.LENGTH_SHORT).show()
                return@setOnClickListener
            }

            btnRegister.isEnabled = false
            progressBar.visibility = View.VISIBLE

            lifecycleScope.launch {
                try {
                    val resp = withContext(Dispatchers.IO) {
                        RetrofitClient.getInstance(this@RegisterActivity)
                            .register(RegisterRequest(username, email, phone, password, fullName))
                    }
                    if (resp.isSuccessful && resp.body() != null) {
                        val body = resp.body()!!
                        SessionManager(this@RegisterActivity).saveLogin(body.token, body.userId, body.username, body.fullName, body.profilePhotoUrl)
                        startActivity(Intent(this@RegisterActivity, MainActivity::class.java).apply {
                            flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
                        })
                        finish()
                    } else {
                        Toast.makeText(this@RegisterActivity, "Registration failed", Toast.LENGTH_SHORT).show()
                    }
                } catch (e: Exception) {
                    Toast.makeText(this@RegisterActivity, "Network error: ${e.message}", Toast.LENGTH_SHORT).show()
                } finally {
                    btnRegister.isEnabled = true
                    progressBar.visibility = View.GONE
                }
            }
        }

        tvLogin.setOnClickListener { finish() }
    }
}
