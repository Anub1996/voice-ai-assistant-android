package com.olxclone.app.utils

import android.content.Context

class SessionManager(context: Context) {
    private val prefs = context.getSharedPreferences("olx_session", Context.MODE_PRIVATE)

    fun saveLogin(token: String, userId: Int, username: String, fullName: String, photoUrl: String?) {
        prefs.edit().putString("token", token).putInt("user_id", userId)
            .putString("username", username).putString("full_name", fullName)
            .putString("photo_url", photoUrl).apply()
    }

    fun getToken(): String? = prefs.getString("token", null)
    fun getUserId(): Int = prefs.getInt("user_id", 0)
    fun getUsername(): String = prefs.getString("username", "") ?: ""
    fun getFullName(): String = prefs.getString("full_name", "") ?: ""
    fun getPhotoUrl(): String? = prefs.getString("photo_url", null)
    fun isLoggedIn(): Boolean = getToken() != null

    fun logout() = prefs.edit().clear().apply()
}
