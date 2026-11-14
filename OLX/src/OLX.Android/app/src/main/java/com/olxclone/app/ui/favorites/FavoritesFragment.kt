package com.olxclone.app.ui.favorites

import android.content.Intent
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import android.widget.Toast
import androidx.fragment.app.Fragment
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.olxclone.app.R
import com.olxclone.app.adapters.FavoriteAdapter
import com.olxclone.app.api.RetrofitClient
import com.olxclone.app.ui.listing.ListingDetailActivity
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class FavoritesFragment : Fragment() {
    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, savedInstanceState: Bundle?): View {
        val view = inflater.inflate(R.layout.fragment_favorites, container, false)

        val rvFavorites = view.findViewById<RecyclerView>(R.id.rvFavorites)
        val tvEmpty = view.findViewById<TextView>(R.id.tvEmpty)
        val api = RetrofitClient.getInstance(requireContext())

        rvFavorites.layoutManager = LinearLayoutManager(context)

        lifecycleScope.launch {
            try {
                val resp = withContext(Dispatchers.IO) { api.getFavorites() }
                if (resp.isSuccessful) {
                    val items = resp.body() ?: emptyList()
                    if (items.isEmpty()) {
                        tvEmpty.visibility = View.VISIBLE
                    } else {
                        val adapter = FavoriteAdapter(
                            onClick = { fav ->
                                startActivity(Intent(requireContext(), ListingDetailActivity::class.java)
                                    .putExtra("listing_id", fav.listingId))
                            },
                            onRemove = { fav ->
                                lifecycleScope.launch {
                                    try {
                                        withContext(Dispatchers.IO) { api.removeFavorite(fav.listingId) }
                                    } catch (_: Exception) {}
                                }
                            })
                        adapter.updateData(items)
                        rvFavorites.adapter = adapter
                    }
                }
            } catch (e: Exception) {
                Toast.makeText(context, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            }
        }

        return view
    }
}
