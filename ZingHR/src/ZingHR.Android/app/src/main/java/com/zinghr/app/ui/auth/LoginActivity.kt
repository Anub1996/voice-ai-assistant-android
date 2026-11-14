package com.zinghr.app.ui.auth

import android.content.Intent
import android.os.Bundle
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.zinghr.app.api.LoginRequest
import com.zinghr.app.api.RetrofitClient
import com.zinghr.app.databinding.ActivityLoginBinding
import com.zinghr.app.ui.dashboard.MainActivity
import com.zinghr.app.utils.SessionManager
import kotlinx.coroutines.launch

class LoginActivity : AppCompatActivity() {

    private lateinit var binding: ActivityLoginBinding
    private lateinit var session: SessionManager

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityLoginBinding.inflate(layoutInflater)
        setContentView(binding.root)

        session = SessionManager(this)

        if (session.isLoggedIn()) {
            navigateToMain()
            return
        }

        binding.btnLogin.setOnClickListener { performLogin() }
        binding.tvHint.text = "Demo: admin/admin123 or amit/emp123"
    }

    private fun performLogin() {
        val username = binding.etUsername.text.toString().trim()
        val password = binding.etPassword.text.toString().trim()

        if (username.isEmpty() || password.isEmpty()) {
            Toast.makeText(this, "Enter username and password", Toast.LENGTH_SHORT).show()
            return
        }

        binding.progressBar.visibility = View.VISIBLE
        binding.btnLogin.isEnabled = false

        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@LoginActivity)
                val response = api.login(LoginRequest(username, password))

                if (response.isSuccessful && response.body() != null) {
                    val body = response.body()!!
                    session.saveLogin(body.token, body.username, body.fullName, body.role, body.employeeId)
                    RetrofitClient.reset()
                    RetrofitClient.init(this@LoginActivity, session.getServerUrl())
                    navigateToMain()
                } else {
                    Toast.makeText(this@LoginActivity, "Invalid credentials", Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                Toast.makeText(this@LoginActivity, "Connection error: ${e.message}", Toast.LENGTH_LONG).show()
            } finally {
                binding.progressBar.visibility = View.GONE
                binding.btnLogin.isEnabled = true
            }
        }
    }

    private fun navigateToMain() {
        startActivity(Intent(this, MainActivity::class.java))
        finish()
    }
}
