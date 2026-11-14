package com.olxclone.app.ui.post

import android.os.Bundle
import android.view.View
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.olxclone.app.R
import com.olxclone.app.api.Category
import com.olxclone.app.api.CreateListingRequest
import com.olxclone.app.api.RetrofitClient
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class PostListingActivity : AppCompatActivity() {
    private var categories = listOf<Category>()
    private var allCats = mutableListOf<Category>()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_post_listing)

        val api = RetrofitClient.getInstance(this)
        val etTitle = findViewById<EditText>(R.id.etTitle)
        val etDescription = findViewById<EditText>(R.id.etDescription)
        val etPrice = findViewById<EditText>(R.id.etPrice)
        val cbNegotiable = findViewById<CheckBox>(R.id.cbNegotiable)
        val spCategory = findViewById<Spinner>(R.id.spCategory)
        val spCondition = findViewById<Spinner>(R.id.spCondition)
        val etCity = findViewById<EditText>(R.id.etCity)
        val etState = findViewById<EditText>(R.id.etState)
        val btnPost = findViewById<Button>(R.id.btnPost)
        val btnBack = findViewById<ImageButton>(R.id.btnBack)
        val progressBar = findViewById<ProgressBar>(R.id.progressBar)

        btnBack.setOnClickListener { finish() }

        val conditions = listOf("Select condition", "new", "like_new", "used", "fair")
        spCondition.adapter = ArrayAdapter(this, android.R.layout.simple_spinner_dropdown_item, conditions)

        lifecycleScope.launch {
            try {
                val resp = withContext(Dispatchers.IO) { api.getCategories() }
                if (resp.isSuccessful) {
                    categories = resp.body() ?: emptyList()
                    allCats.clear()
                    allCats.add(Category(0, "Select category", "", "", null, null))
                    categories.forEach { c ->
                        allCats.add(c)
                        c.children?.forEach { ch -> allCats.add(ch) }
                    }
                    spCategory.adapter = ArrayAdapter(this@PostListingActivity,
                        android.R.layout.simple_spinner_dropdown_item, allCats.map { it.name })
                }
            } catch (_: Exception) {}
        }

        btnPost.setOnClickListener {
            val title = etTitle.text.toString().trim()
            val desc = etDescription.text.toString().trim()
            val priceStr = etPrice.text.toString().trim()
            val condPos = spCondition.selectedItemPosition
            val catPos = spCategory.selectedItemPosition

            if (title.isEmpty() || desc.isEmpty() || priceStr.isEmpty() || condPos == 0 || catPos == 0) {
                Toast.makeText(this, "Please fill all required fields", Toast.LENGTH_SHORT).show()
                return@setOnClickListener
            }

            btnPost.isEnabled = false
            progressBar.visibility = View.VISIBLE

            val req = CreateListingRequest(
                title = title, description = desc, price = priceStr.toDouble(),
                currency = "INR", isNegotiable = cbNegotiable.isChecked,
                condition = conditions[condPos], categoryId = allCats[catPos].id,
                latitude = null, longitude = null, address = null,
                city = etCity.text.toString().trim().ifEmpty { null },
                state = etState.text.toString().trim().ifEmpty { null },
                imageUrls = null
            )

            lifecycleScope.launch {
                try {
                    val resp = withContext(Dispatchers.IO) { api.createListing(req) }
                    if (resp.isSuccessful) {
                        Toast.makeText(this@PostListingActivity, "Ad posted!", Toast.LENGTH_SHORT).show()
                        finish()
                    } else {
                        Toast.makeText(this@PostListingActivity, "Failed to post ad", Toast.LENGTH_SHORT).show()
                    }
                } catch (e: Exception) {
                    Toast.makeText(this@PostListingActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
                } finally {
                    btnPost.isEnabled = true
                    progressBar.visibility = View.GONE
                }
            }
        }
    }
}
