package com.olxclone.app.ui.home

import android.content.Intent
import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import androidx.fragment.app.Fragment
import com.google.android.material.bottomnavigation.BottomNavigationView
import com.olxclone.app.R
import com.olxclone.app.ui.chat.ChatListFragment
import com.olxclone.app.ui.favorites.FavoritesFragment
import com.olxclone.app.ui.post.PostListingActivity
import com.olxclone.app.ui.profile.ProfileFragment

class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val bottomNav = findViewById<BottomNavigationView>(R.id.bottomNav)

        if (savedInstanceState == null) loadFragment(HomeFragment())

        bottomNav.setOnItemSelectedListener { item ->
            when (item.itemId) {
                R.id.nav_home -> { loadFragment(HomeFragment()); true }
                R.id.nav_chat -> { loadFragment(ChatListFragment()); true }
                R.id.nav_sell -> { startActivity(Intent(this, PostListingActivity::class.java)); false }
                R.id.nav_favorites -> { loadFragment(FavoritesFragment()); true }
                R.id.nav_profile -> { loadFragment(ProfileFragment()); true }
                else -> false
            }
        }
    }

    private fun loadFragment(fragment: Fragment) {
        supportFragmentManager.beginTransaction()
            .replace(R.id.fragmentContainer, fragment)
            .commit()
    }
}
