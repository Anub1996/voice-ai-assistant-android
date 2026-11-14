package com.olxclone.app.ui.listing

import android.content.Intent
import android.os.Bundle
import android.view.View
import android.widget.ImageButton
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.olxclone.app.R
import com.olxclone.app.api.RetrofitClient
import com.olxclone.app.api.StartConversationRequest
import com.olxclone.app.ui.chat.ChatActivity
import com.olxclone.app.utils.SessionManager
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class ListingDetailActivity : AppCompatActivity() {
    private var listingId = 0L
    private var isFavorited = false

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_listing_detail)

        listingId = intent.getLongExtra("listing_id", 0)
        val api = RetrofitClient.getInstance(this)

        val btnBack = findViewById<ImageButton>(R.id.btnBack)
        val btnFavorite = findViewById<ImageButton>(R.id.btnFavorite)
        val btnShare = findViewById<ImageButton>(R.id.btnShare)
        val tvPrice = findViewById<TextView>(R.id.tvPrice)
        val tvTitle = findViewById<TextView>(R.id.tvTitle)
        val tvCondition = findViewById<TextView>(R.id.tvCondition)
        val tvNegotiable = findViewById<TextView>(R.id.tvNegotiable)
        val tvLocation = findViewById<TextView>(R.id.tvLocation)
        val tvPostedDate = findViewById<TextView>(R.id.tvPostedDate)
        val tvViews = findViewById<TextView>(R.id.tvViews)
        val tvDescription = findViewById<TextView>(R.id.tvDescription)
        val tvSellerName = findViewById<TextView>(R.id.tvSellerName)
        val tvSellerJoined = findViewById<TextView>(R.id.tvSellerJoined)
        val btnChat = findViewById<View>(R.id.btnChat)
        val btnMakeOffer = findViewById<View>(R.id.btnMakeOffer)

        btnBack.setOnClickListener { finish() }

        lifecycleScope.launch {
            try {
                val resp = withContext(Dispatchers.IO) { api.getListingDetail(listingId) }
                if (resp.isSuccessful && resp.body() != null) {
                    val d = resp.body()!!
                    tvPrice.text = "₹%,.0f".format(d.price)
                    tvTitle.text = d.title
                    tvCondition.text = d.condition.replace("_", " ").uppercase()
                    if (d.isNegotiable) {
                        tvNegotiable.text = "NEGOTIABLE"
                        tvNegotiable.visibility = View.VISIBLE
                    }
                    tvLocation.text = listOfNotNull(d.city, d.state).joinToString(", ")
                    tvPostedDate.text = "Posted: ${d.createdAt.take(10)}"
                    tvViews.text = "${d.viewCount} views"
                    tvDescription.text = d.description
                    tvSellerName.text = d.seller.fullName
                    tvSellerJoined.text = "Member since ${d.seller.joinedAt.take(10)}"
                    isFavorited = d.isFavorited
                    updateFavIcon(btnFavorite)

                    btnShare.setOnClickListener {
                        val shareIntent = Intent(Intent.ACTION_SEND).apply {
                            type = "text/plain"
                            putExtra(Intent.EXTRA_TEXT, "${d.title} - ₹%,.0f".format(d.price))
                        }
                        startActivity(Intent.createChooser(shareIntent, "Share via"))
                    }

                    btnChat.setOnClickListener {
                        lifecycleScope.launch {
                            try {
                                val chatResp = withContext(Dispatchers.IO) {
                                    api.startConversation(StartConversationRequest(listingId, "Hi, is this still available?"))
                                }
                                val convId = if (chatResp.isSuccessful) chatResp.body()?.id ?: 0 else {
                                    val errorBody = chatResp.errorBody()?.string() ?: ""
                                    if (chatResp.code() == 409) {
                                        errorBody.substringAfter("\"id\":").substringBefore("}").trim().toLongOrNull() ?: 0
                                    } else 0
                                }
                                if (convId > 0) {
                                    startActivity(Intent(this@ListingDetailActivity, ChatActivity::class.java)
                                        .putExtra("conversation_id", convId)
                                        .putExtra("title", d.title))
                                }
                            } catch (e: Exception) {
                                Toast.makeText(this@ListingDetailActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
                            }
                        }
                    }

                    btnMakeOffer.setOnClickListener {
                        btnChat.performClick()
                    }
                }
            } catch (e: Exception) {
                Toast.makeText(this@ListingDetailActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            }
        }

        btnFavorite.setOnClickListener {
            lifecycleScope.launch {
                try {
                    withContext(Dispatchers.IO) {
                        if (isFavorited) api.removeFavorite(listingId) else api.addFavorite(listingId)
                    }
                    isFavorited = !isFavorited
                    updateFavIcon(btnFavorite)
                } catch (_: Exception) {}
            }
        }
    }

    private fun updateFavIcon(btn: ImageButton) {
        btn.setImageResource(
            if (isFavorited) android.R.drawable.btn_star_big_on else android.R.drawable.btn_star_big_off
        )
    }
}
