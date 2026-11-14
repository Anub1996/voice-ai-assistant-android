package com.olxclone.app.ui.chat

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
import com.olxclone.app.adapters.ConversationAdapter
import com.olxclone.app.api.RetrofitClient
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class ChatListFragment : Fragment() {
    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, savedInstanceState: Bundle?): View {
        val view = inflater.inflate(R.layout.fragment_chat_list, container, false)

        val rvConversations = view.findViewById<RecyclerView>(R.id.rvConversations)
        val tvEmpty = view.findViewById<TextView>(R.id.tvEmpty)

        rvConversations.layoutManager = LinearLayoutManager(context)

        lifecycleScope.launch {
            try {
                val resp = withContext(Dispatchers.IO) {
                    RetrofitClient.getInstance(requireContext()).getConversations()
                }
                if (resp.isSuccessful) {
                    val items = resp.body() ?: emptyList()
                    if (items.isEmpty()) {
                        tvEmpty.visibility = View.VISIBLE
                    } else {
                        rvConversations.adapter = ConversationAdapter(items) { convo ->
                            startActivity(Intent(requireContext(), ChatActivity::class.java)
                                .putExtra("conversation_id", convo.id)
                                .putExtra("title", "${convo.otherUserName} - ${convo.listingTitle}"))
                        }
                    }
                }
            } catch (e: Exception) {
                Toast.makeText(context, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            }
        }

        return view
    }
}
