package com.olxclone.app.ui.search

import android.content.Intent
import android.os.Bundle
import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.EditText
import android.widget.ImageButton
import android.widget.ProgressBar
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.GridLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.olxclone.app.R
import com.olxclone.app.adapters.ListingAdapter
import com.olxclone.app.api.RetrofitClient
import com.olxclone.app.ui.listing.ListingDetailActivity
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class SearchActivity : AppCompatActivity() {
    private var categoryId: Int? = null
    private var sortBy: String? = null
    private var condition: String? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_search)

        val api = RetrofitClient.getInstance(this)
        val btnBack = findViewById<ImageButton>(R.id.btnBack)
        val etSearch = findViewById<EditText>(R.id.etSearch)
        val progressBar = findViewById<ProgressBar>(R.id.progressBar)
        val tvResultCount = findViewById<TextView>(R.id.tvResultCount)
        val rvResults = findViewById<RecyclerView>(R.id.rvResults)

        rvResults.layoutManager = GridLayoutManager(this, 2)
        btnBack.setOnClickListener { finish() }

        categoryId = intent.getIntExtra("category_id", 0).takeIf { it > 0 }
        val categoryName = intent.getStringExtra("category_name")
        if (categoryName != null) etSearch.hint = categoryName

        fun search() {
            val query = etSearch.text.toString().trim().ifEmpty { null }
            progressBar.visibility = View.VISIBLE

            lifecycleScope.launch {
                try {
                    val resp = withContext(Dispatchers.IO) {
                        api.getListings(query = query, categoryId = categoryId,
                            condition = condition, sortBy = sortBy)
                    }
                    if (resp.isSuccessful && resp.body() != null) {
                        val result = resp.body()!!
                        tvResultCount.text = "${result.total} results"
                        val adapter = ListingAdapter({ listing ->
                            startActivity(Intent(this@SearchActivity, ListingDetailActivity::class.java)
                                .putExtra("listing_id", listing.id))
                        })
                        adapter.updateData(result.data)
                        rvResults.adapter = adapter
                    }
                } catch (e: Exception) {
                    Toast.makeText(this@SearchActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
                } finally {
                    progressBar.visibility = View.GONE
                }
            }
        }

        etSearch.setOnEditorActionListener { _, actionId, _ ->
            if (actionId == EditorInfo.IME_ACTION_SEARCH) { search(); true } else false
        }

        findViewById<View>(R.id.chipSort).setOnClickListener {
            sortBy = if (sortBy == "price_asc") "price_desc" else "price_asc"
            search()
        }

        findViewById<View>(R.id.chipCondition).setOnClickListener {
            condition = when (condition) {
                null -> "new"
                "new" -> "like_new"
                "like_new" -> "used"
                else -> null
            }
            search()
        }

        if (categoryId != null) search()

        etSearch.requestFocus()
    }
}
