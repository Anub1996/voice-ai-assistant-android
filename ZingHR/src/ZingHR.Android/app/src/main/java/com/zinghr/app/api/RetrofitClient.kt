package com.zinghr.app.api

import android.content.Context
import com.zinghr.app.utils.SessionManager
import okhttp3.Interceptor
import okhttp3.OkHttpClient
import okhttp3.logging.HttpLoggingInterceptor
import retrofit2.Retrofit
import retrofit2.converter.gson.GsonConverterFactory
import java.util.concurrent.TimeUnit

object RetrofitClient {

    private var api: ZingHRApi? = null
    private var baseUrl: String = "http://10.0.2.2:5000/"

    fun init(context: Context, url: String? = null) {
        if (url != null) baseUrl = url
        api = createApi(context)
    }

    fun getApi(context: Context): ZingHRApi {
        if (api == null) init(context)
        return api!!
    }

    private fun createApi(context: Context): ZingHRApi {
        val session = SessionManager(context)

        val authInterceptor = Interceptor { chain ->
            val request = chain.request().newBuilder()
            session.getToken()?.let { token ->
                request.addHeader("Authorization", "Bearer $token")
            }
            chain.proceed(request.build())
        }

        val logging = HttpLoggingInterceptor().apply {
            level = HttpLoggingInterceptor.Level.BODY
        }

        val client = OkHttpClient.Builder()
            .addInterceptor(authInterceptor)
            .addInterceptor(logging)
            .connectTimeout(30, TimeUnit.SECONDS)
            .readTimeout(30, TimeUnit.SECONDS)
            .writeTimeout(30, TimeUnit.SECONDS)
            .build()

        return Retrofit.Builder()
            .baseUrl(baseUrl)
            .client(client)
            .addConverterFactory(GsonConverterFactory.create())
            .build()
            .create(ZingHRApi::class.java)
    }

    fun reset() {
        api = null
    }
}
