package com.zinghr.app.ui.directory

import android.os.Bundle
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.appcompat.widget.SearchView
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import com.zinghr.app.adapters.DirectoryAdapter
import com.zinghr.app.api.RetrofitClient
import com.zinghr.app.databinding.ActivityDirectoryBinding
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

class DirectoryActivity : AppCompatActivity() {

    private lateinit var binding: ActivityDirectoryBinding
    private var searchJob: Job? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityDirectoryBinding.inflate(layoutInflater)
        setContentView(binding.root)

        binding.toolbar.setNavigationOnClickListener { finish() }
        binding.rvDirectory.layoutManager = LinearLayoutManager(this)

        binding.searchView.setOnQueryTextListener(object : SearchView.OnQueryTextListener {
            override fun onQueryTextSubmit(query: String?) = false.also { loadDirectory(query) }
            override fun onQueryTextChange(newText: String?): Boolean {
                searchJob?.cancel()
                searchJob = lifecycleScope.launch {
                    delay(400)
                    loadDirectory(newText)
                }
                return true
            }
        })

        loadDirectory(null)
    }

    private fun loadDirectory(search: String?) {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@DirectoryActivity)
                val response = api.getDirectory(search = search?.takeIf { it.isNotBlank() })
                if (response.isSuccessful && response.body() != null) {
                    val list = response.body()!!
                    if (list.isEmpty()) {
                        binding.tvEmpty.visibility = View.VISIBLE
                        binding.rvDirectory.visibility = View.GONE
                    } else {
                        binding.tvEmpty.visibility = View.GONE
                        binding.rvDirectory.visibility = View.VISIBLE
                        binding.rvDirectory.adapter = DirectoryAdapter(list)
                    }
                }
            } catch (e: Exception) {
                Toast.makeText(this@DirectoryActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            }
        }
    }
}
