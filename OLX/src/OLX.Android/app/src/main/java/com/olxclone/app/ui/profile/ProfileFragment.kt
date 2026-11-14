package com.olxclone.app.ui.profile

import android.content.Intent
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import android.widget.Toast
import androidx.fragment.app.Fragment
import androidx.lifecycle.lifecycleScope
import com.olxclone.app.R
import com.olxclone.app.api.RetrofitClient
import com.olxclone.app.ui.auth.LoginActivity
import com.olxclone.app.utils.SessionManager
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class ProfileFragment : Fragment() {
    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, savedInstanceState: Bundle?): View {
        val view = inflater.inflate(R.layout.fragment_profile, container, false)

        val session = SessionManager(requireContext())
        val tvName = view.findViewById<TextView>(R.id.tvName)
        val tvEmail = view.findViewById<TextView>(R.id.tvEmail)
        val tvLocation = view.findViewById<TextView>(R.id.tvLocation)
        val tvAdsCount = view.findViewById<TextView>(R.id.tvAdsCount)

        tvName.text = session.getFullName()

        lifecycleScope.launch {
            try {
                val resp = withContext(Dispatchers.IO) {
                    RetrofitClient.getInstance(requireContext()).getProfile()
                }
                if (resp.isSuccessful && resp.body() != null) {
                    val p = resp.body()!!
                    tvName.text = p.fullName
                    tvEmail.text = p.email
                    tvLocation.text = listOfNotNull(p.city, p.state).joinToString(", ")
                    tvAdsCount.text = "${p.listingsCount}"
                }
            } catch (_: Exception) {}
        }

        view.findViewById<View>(R.id.btnMyAds).setOnClickListener {
            startActivity(Intent(requireContext(), MyAdsActivity::class.java))
        }

        view.findViewById<View>(R.id.btnNotifications).setOnClickListener {
            startActivity(Intent(requireContext(), NotificationsActivity::class.java))
        }

        view.findViewById<View>(R.id.btnSavedSearches).setOnClickListener {
            Toast.makeText(context, "Saved searches", Toast.LENGTH_SHORT).show()
        }

        view.findViewById<View>(R.id.btnEditProfile).setOnClickListener {
            Toast.makeText(context, "Edit profile", Toast.LENGTH_SHORT).show()
        }

        view.findViewById<View>(R.id.btnLogout).setOnClickListener {
            session.logout()
            startActivity(Intent(requireContext(), LoginActivity::class.java).apply {
                flags = Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK
            })
        }

        return view
    }
}
