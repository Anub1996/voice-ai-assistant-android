package com.olxclone.app.api

import android.content.Context
import com.olxclone.app.utils.SessionManager
import okhttp3.Interceptor
import okhttp3.OkHttpClient
import retrofit2.Retrofit
import retrofit2.converter.gson.GsonConverterFactory
import java.util.concurrent.TimeUnit

object RetrofitClient {
    private const val BASE_URL = "http://10.0.2.2:5000/"
    private var api: ApiService? = null

    fun getInstance(context: Context): ApiService {
        if (api == null) {
            val client = OkHttpClient.Builder()
                .connectTimeout(30, TimeUnit.SECONDS)
                .readTimeout(30, TimeUnit.SECONDS)
                .addInterceptor(Interceptor { chain ->
                    val token = SessionManager(context).getToken()
                    val req = chain.request().newBuilder()
                    if (token != null) req.addHeader("Authorization", "Bearer $token")
                    chain.proceed(req.build())
                }).build()

            api = Retrofit.Builder()
                .baseUrl(BASE_URL)
                .client(client)
                .addConverterFactory(GsonConverterFactory.create())
                .build()
                .create(ApiService::class.java)
        }
        return api!!
    }
}
