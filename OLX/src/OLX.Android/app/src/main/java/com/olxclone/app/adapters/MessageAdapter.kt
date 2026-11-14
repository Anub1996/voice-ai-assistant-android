package com.olxclone.app.adapters

import android.view.Gravity
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.content.ContextCompat
import androidx.recyclerview.widget.RecyclerView
import com.olxclone.app.R
import com.olxclone.app.api.ChatMessage

class MessageAdapter(
    private val currentUserId: Int
) : RecyclerView.Adapter<MessageAdapter.VH>() {

    private val items = mutableListOf<ChatMessage>()

    fun updateData(data: List<ChatMessage>) {
        items.clear()
        items.addAll(data)
        notifyDataSetChanged()
    }

    fun addMessage(msg: ChatMessage) {
        items.add(msg)
        notifyItemInserted(items.size - 1)
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int) =
        VH(LayoutInflater.from(parent.context).inflate(R.layout.item_message, parent, false))

    override fun onBindViewHolder(holder: VH, position: Int) = holder.bind(items[position])
    override fun getItemCount() = items.size

    inner class VH(v: View) : RecyclerView.ViewHolder(v) {
        private val bubbleLayout: LinearLayout = v.findViewById(R.id.bubbleLayout)
        private val tvText: TextView = v.findViewById(R.id.tvText)
        private val tvTime: TextView = v.findViewById(R.id.tvTime)

        fun bind(item: ChatMessage) {
            tvText.text = item.text
            tvTime.text = item.sentAt.substringAfter("T").take(5)

            val isSent = item.senderId == currentUserId
            val lp = bubbleLayout.layoutParams as FrameLayout.LayoutParams
            if (isSent) {
                lp.gravity = Gravity.END
                bubbleLayout.setBackgroundColor(ContextCompat.getColor(itemView.context, R.color.chat_sent))
            } else {
                lp.gravity = Gravity.START
                bubbleLayout.setBackgroundColor(ContextCompat.getColor(itemView.context, R.color.chat_received))
            }
            bubbleLayout.layoutParams = lp
        }
    }
}
