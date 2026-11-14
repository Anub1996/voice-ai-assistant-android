package com.zinghr.app.adapters

import android.graphics.Typeface
import android.view.LayoutInflater
import android.view.ViewGroup
import androidx.recyclerview.widget.RecyclerView
import com.zinghr.app.api.NotificationDto
import com.zinghr.app.databinding.ItemNotificationBinding

class NotificationAdapter(
    private val items: List<NotificationDto>,
    private val onRead: (Long) -> Unit
) : RecyclerView.Adapter<NotificationAdapter.VH>() {

    inner class VH(val b: ItemNotificationBinding) : RecyclerView.ViewHolder(b.root)

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int) =
        VH(ItemNotificationBinding.inflate(LayoutInflater.from(parent.context), parent, false))

    override fun onBindViewHolder(holder: VH, position: Int) {
        val item = items[position]
        holder.b.tvTitle.text = item.title
        holder.b.tvBody.text = item.body
        holder.b.tvTime.text = item.createdAt.take(16).replace("T", " ")
        holder.b.tvType.text = item.type.replaceFirstChar { it.uppercase() }
        if (!item.isRead) {
            holder.b.tvTitle.setTypeface(null, Typeface.BOLD)
            holder.itemView.setOnClickListener { onRead(item.id) }
        } else {
            holder.b.tvTitle.setTypeface(null, Typeface.NORMAL)
        }
    }

    override fun getItemCount() = items.size
}
