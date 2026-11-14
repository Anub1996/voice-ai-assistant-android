package com.olxclone.app.adapters

import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.TextView
import androidx.core.view.isVisible
import androidx.recyclerview.widget.RecyclerView
import com.olxclone.app.R
import com.olxclone.app.api.Conversation

class ConversationAdapter(
    private val onClick: (Conversation) -> Unit
) : RecyclerView.Adapter<ConversationAdapter.VH>() {

    private val items = mutableListOf<Conversation>()

    fun updateData(data: List<Conversation>) {
        items.clear()
        items.addAll(data)
        notifyDataSetChanged()
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int) =
        VH(LayoutInflater.from(parent.context).inflate(R.layout.item_conversation, parent, false))

    override fun onBindViewHolder(holder: VH, position: Int) = holder.bind(items[position])
    override fun getItemCount() = items.size

    inner class VH(v: View) : RecyclerView.ViewHolder(v) {
        private val ivListingThumb: ImageView = v.findViewById(R.id.ivListingThumb)
        private val tvUserName: TextView = v.findViewById(R.id.tvUserName)
        private val tvListingTitle: TextView = v.findViewById(R.id.tvListingTitle)
        private val tvLastMessage: TextView = v.findViewById(R.id.tvLastMessage)
        private val tvTime: TextView = v.findViewById(R.id.tvTime)
        private val tvUnread: TextView = v.findViewById(R.id.tvUnread)

        fun bind(item: Conversation) {
            tvUserName.text = item.otherUserName
            tvListingTitle.text = item.listingTitle
            tvLastMessage.text = item.lastMessage ?: ""
            tvTime.text = item.lastMessageAt?.take(10) ?: ""
            tvUnread.isVisible = item.unreadCount > 0
            if (item.unreadCount > 0) tvUnread.text = item.unreadCount.toString()
            itemView.setOnClickListener { onClick(item) }
        }
    }
}
