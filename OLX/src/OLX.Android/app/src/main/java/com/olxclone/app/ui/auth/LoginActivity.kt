package com.olxclone.app.ui.auth

import android.content.Intent
import android.os.Bundle
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.olxclone.app.R
import com.olxclone.app.api.LoginRequest
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

class LoginActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        if (SessionManager(this).isLoggedIn()) {
            startActivity(Intent(this, MainActivity::class.java))
            finish()
            return
        }

        setContentView(R.layout.activity_login)

        val etUsername = findViewById<EditText>(R.id.etUsername)
        val etPassword = findViewById<EditText>(R.id.etPassword)
        val btnLogin = findViewById<Button>(R.id.btnLogin)
        val tvRegister = findViewById<TextView>(R.id.tvRegister)
        val progressBar = findViewById<ProgressBar>(R.id.progressBar)

        btnLogin.setOnClickListener {
            val username = etUsername.text.toString().trim()
            val password = etPassword.text.toString().trim()
            if (username.isEmpty() || password.isEmpty()) {
                Toast.makeText(this, "Please fill all fields", Toast.LENGTH_SHORT).show()
                return@setOnClickListener
            }

            btnLogin.isEnabled = false
            progressBar.visibility = View.VISIBLE

            lifecycleScope.launch {
                try {
                    val resp = withContext(Dispatchers.IO) {
                        RetrofitClient.getInstance(this@LoginActivity).login(LoginRequest(username, password))
                    }
                    if (resp.isSuccessful && resp.body() != null) {
                        val body = resp.body()!!
                        SessionManager(this@LoginActivity).saveLogin(body.token, body.userId, body.username, body.fullName, body.profilePhotoUrl)
                        startActivity(Intent(this@LoginActivity, MainActivity::class.java))
                        finish()
                    } else {
                        Toast.makeText(this@LoginActivity, "Invalid credentials", Toast.LENGTH_SHORT).show()
                    }
                } catch (e: Exception) {
                    Toast.makeText(this@LoginActivity, "Network error: ${e.message}", Toast.LENGTH_SHORT).show()
                } finally {
                    btnLogin.isEnabled = true
                    progressBar.visibility = View.GONE
                }
            }
        }

        tvRegister.setOnClickListener {
            startActivity(Intent(this, RegisterActivity::class.java))
        }
    }
}
