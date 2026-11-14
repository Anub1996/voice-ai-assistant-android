package com.olxclone.app.ui.chat

import android.os.Bundle
import android.widget.EditText
import android.widget.ImageButton
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import com.olxclone.app.R
import com.olxclone.app.adapters.MessageAdapter
import com.olxclone.app.api.RetrofitClient
import com.olxclone.app.api.SendMessageRequest
import com.olxclone.app.utils.SessionManager
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class ChatActivity : AppCompatActivity() {
    private var conversationId = 0L

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_chat)

        conversationId = intent.getLongExtra("conversation_id", 0)
        val title = intent.getStringExtra("title") ?: "Chat"
        val api = RetrofitClient.getInstance(this)
        val myUserId = SessionManager(this).getUserId()

        val btnBack = findViewById<ImageButton>(R.id.btnBack)
        val tvChatTitle = findViewById<TextView>(R.id.tvChatTitle)
        val rvMessages = findViewById<RecyclerView>(R.id.rvMessages)
        val etMessage = findViewById<EditText>(R.id.etMessage)
        val btnSend = findViewById<ImageButton>(R.id.btnSend)

        tvChatTitle.text = title
        btnBack.setOnClickListener { finish() }

        val lm = LinearLayoutManager(this).apply { stackFromEnd = true }
        rvMessages.layoutManager = lm

        fun loadMessages() {
            lifecycleScope.launch {
                try {
                    val resp = withContext(Dispatchers.IO) { api.getMessages(conversationId) }
                    if (resp.isSuccessful) {
                        val msgs = resp.body() ?: emptyList()
                        val adapter = MessageAdapter(myUserId)
                        adapter.updateData(msgs)
                        rvMessages.adapter = adapter
                        if (msgs.isNotEmpty()) rvMessages.scrollToPosition(msgs.size - 1)
                    }
                } catch (e: Exception) {
                    Toast.makeText(this@ChatActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
                }
            }
        }

        loadMessages()

        btnSend.setOnClickListener {
            val text = etMessage.text.toString().trim()
            if (text.isEmpty()) return@setOnClickListener

            etMessage.text.clear()
            lifecycleScope.launch {
                try {
                    withContext(Dispatchers.IO) {
                        api.sendMessage(conversationId, SendMessageRequest(text, null))
                    }
                    loadMessages()
                } catch (e: Exception) {
                    Toast.makeText(this@ChatActivity, "Failed to send", Toast.LENGTH_SHORT).show()
                }
            }
        }
    }
}
