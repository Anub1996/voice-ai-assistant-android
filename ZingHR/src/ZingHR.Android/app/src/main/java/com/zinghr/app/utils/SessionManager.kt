package com.zinghr.app.utils

import android.content.Context
import android.content.SharedPreferences

class SessionManager(context: Context) {

    private val prefs: SharedPreferences =
        context.getSharedPreferences("zinghr_session", Context.MODE_PRIVATE)

    fun saveLogin(token: String, username: String, fullName: String, role: String, employeeId: Int?) {
        prefs.edit().apply {
            putString(KEY_TOKEN, token)
            putString(KEY_USERNAME, username)
            putString(KEY_FULL_NAME, fullName)
            putString(KEY_ROLE, role)
            employeeId?.let { putInt(KEY_EMPLOYEE_ID, it) }
            putBoolean(KEY_IS_LOGGED_IN, true)
            apply()
        }
    }

    fun getToken(): String? = prefs.getString(KEY_TOKEN, null)
    fun getUsername(): String = prefs.getString(KEY_USERNAME, "") ?: ""
    fun getFullName(): String = prefs.getString(KEY_FULL_NAME, "") ?: ""
    fun getRole(): String = prefs.getString(KEY_ROLE, "") ?: ""
    fun getEmployeeId(): Int = prefs.getInt(KEY_EMPLOYEE_ID, 0)
    fun isLoggedIn(): Boolean = prefs.getBoolean(KEY_IS_LOGGED_IN, false)

    fun savePin(pin: String) {
        prefs.edit().putString(KEY_PIN, pin).apply()
    }

    fun getPin(): String? = prefs.getString(KEY_PIN, null)

    fun saveServerUrl(url: String) {
        prefs.edit().putString(KEY_SERVER_URL, url).apply()
    }

    fun getServerUrl(): String = prefs.getString(KEY_SERVER_URL, "http://10.0.2.2:5000/") ?: "http://10.0.2.2:5000/"

    fun logout() {
        val serverUrl = getServerUrl()
        prefs.edit().clear().apply()
        saveServerUrl(serverUrl)
    }

    companion object {
        private const val KEY_TOKEN = "token"
        private const val KEY_USERNAME = "username"
        private const val KEY_FULL_NAME = "full_name"
        private const val KEY_ROLE = "role"
        private const val KEY_EMPLOYEE_ID = "employee_id"
        private const val KEY_IS_LOGGED_IN = "is_logged_in"
        private const val KEY_PIN = "pin"
        private const val KEY_SERVER_URL = "server_url"
    }
}
