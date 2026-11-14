package com.olxclone.app.adapters

import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.core.view.isVisible
import androidx.recyclerview.widget.RecyclerView
import com.olxclone.app.R
import com.olxclone.app.api.NotificationItem

class NotificationAdapter(
    private val onClick: (NotificationItem) -> Unit
) : RecyclerView.Adapter<NotificationAdapter.VH>() {

    private val items = mutableListOf<NotificationItem>()

    fun updateData(data: List<NotificationItem>) {
        items.clear()
        items.addAll(data)
        notifyDataSetChanged()
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int) =
        VH(LayoutInflater.from(parent.context).inflate(R.layout.item_notification, parent, false))

    override fun onBindViewHolder(holder: VH, position: Int) = holder.bind(items[position])
    override fun getItemCount() = items.size

    inner class VH(v: View) : RecyclerView.ViewHolder(v) {
        private val dotUnread: View = v.findViewById(R.id.dotUnread)
        private val tvTitle: TextView = v.findViewById(R.id.tvTitle)
        private val tvBody: TextView = v.findViewById(R.id.tvBody)
        private val tvTime: TextView = v.findViewById(R.id.tvTime)

        fun bind(item: NotificationItem) {
            dotUnread.isVisible = !item.isRead
            tvTitle.text = item.title
            tvBody.text = item.body
            tvTime.text = item.createdAt.take(10)
            itemView.setOnClickListener { onClick(item) }
        }
    }
}
