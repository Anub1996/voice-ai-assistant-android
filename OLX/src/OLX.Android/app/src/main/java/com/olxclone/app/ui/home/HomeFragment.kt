package com.olxclone.app.ui.home

import android.content.Intent
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.view.inputmethod.EditorInfo
import android.widget.EditText
import android.widget.ImageButton
import android.widget.Toast
import androidx.fragment.app.Fragment
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.GridLayoutManager
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import androidx.swiperefreshlayout.widget.SwipeRefreshLayout
import com.olxclone.app.R
import com.olxclone.app.adapters.CategoryAdapter
import com.olxclone.app.adapters.ListingAdapter
import com.olxclone.app.api.RetrofitClient
import com.olxclone.app.ui.listing.ListingDetailActivity
import com.olxclone.app.ui.profile.NotificationsActivity
import com.olxclone.app.ui.search.SearchActivity
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class HomeFragment : Fragment() {
    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, savedInstanceState: Bundle?): View {
        val view = inflater.inflate(R.layout.fragment_home, container, false)

        val etSearch = view.findViewById<EditText>(R.id.etSearch)
        val btnNotifications = view.findViewById<ImageButton>(R.id.btnNotifications)
        val rvCategories = view.findViewById<RecyclerView>(R.id.rvCategories)
        val rvListings = view.findViewById<RecyclerView>(R.id.rvListings)
        val swipeRefresh = view.findViewById<SwipeRefreshLayout>(R.id.swipeRefresh)

        val api = RetrofitClient.getInstance(requireContext())

        rvCategories.layoutManager = LinearLayoutManager(context, LinearLayoutManager.HORIZONTAL, false)
        rvListings.layoutManager = GridLayoutManager(context, 2)

        etSearch.isFocusable = false
        etSearch.setOnClickListener {
            startActivity(Intent(requireContext(), SearchActivity::class.java))
        }

        btnNotifications.setOnClickListener {
            startActivity(Intent(requireContext(), NotificationsActivity::class.java))
        }

        fun loadData() {
            lifecycleScope.launch {
                try {
                    val catResp = withContext(Dispatchers.IO) { api.getCategories() }
                    if (catResp.isSuccessful) {
                        val cats = catResp.body() ?: emptyList()
                        rvCategories.adapter = CategoryAdapter(cats) { cat ->
                            startActivity(Intent(requireContext(), SearchActivity::class.java)
                                .putExtra("category_id", cat.id)
                                .putExtra("category_name", cat.name))
                        }
                    }

                    val listResp = withContext(Dispatchers.IO) { api.getListings(pageSize = 20) }
                    if (listResp.isSuccessful) {
                        val items = listResp.body()?.data ?: emptyList()
                        rvListings.adapter = ListingAdapter(items) { listing ->
                            startActivity(Intent(requireContext(), ListingDetailActivity::class.java)
                                .putExtra("listing_id", listing.id))
                        }
                    }
                } catch (e: Exception) {
                    Toast.makeText(context, "Error loading: ${e.message}", Toast.LENGTH_SHORT).show()
                } finally {
                    swipeRefresh.isRefreshing = false
                }
            }
        }

        loadData()
        swipeRefresh.setOnRefreshListener { loadData() }

        return view
    }
}
