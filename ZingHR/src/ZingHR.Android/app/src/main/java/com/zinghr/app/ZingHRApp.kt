package com.zinghr.app

import android.app.Application
import com.zinghr.app.api.RetrofitClient
import com.zinghr.app.utils.SessionManager

class ZingHRApp : Application() {

    override fun onCreate() {
        super.onCreate()
        val session = SessionManager(this)
        RetrofitClient.init(this, session.getServerUrl())
    }
}
