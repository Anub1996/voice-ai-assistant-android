package com.olxclone.app.ui.profile

import android.content.Intent
import android.os.Bundle
import android.view.View
import android.widget.ImageButton
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.olxclone.app.R
import com.olxclone.app.adapters.ListingAdapter
import com.olxclone.app.api.RetrofitClient
import com.olxclone.app.ui.listing.ListingDetailActivity
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class MyAdsActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_my_ads)

        val btnBack = findViewById<ImageButton>(R.id.btnBack)
        val tvEmpty = findViewById<TextView>(R.id.tvEmpty)
        val rvListings = findViewById<RecyclerView>(R.id.rvListings)

        rvListings.layoutManager = LinearLayoutManager(this)
        btnBack.setOnClickListener { finish() }

        lifecycleScope.launch {
            try {
                val resp = withContext(Dispatchers.IO) {
                    RetrofitClient.getInstance(this@MyAdsActivity).getMyListings()
                }
                if (resp.isSuccessful) {
                    val items = resp.body() ?: emptyList()
                    if (items.isEmpty()) {
                        tvEmpty.visibility = View.VISIBLE
                    } else {
                        val adapter = ListingAdapter({ listing ->
                            startActivity(Intent(this@MyAdsActivity, ListingDetailActivity::class.java)
                                .putExtra("listing_id", listing.id))
                        })
                        adapter.updateData(items)
                        rvListings.adapter = adapter
                    }
                }
            } catch (e: Exception) {
                Toast.makeText(this@MyAdsActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            }
        }
    }
}
