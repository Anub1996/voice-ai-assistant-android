package com.zinghr.app.adapters

import android.graphics.Color
import android.view.LayoutInflater
import android.view.ViewGroup
import androidx.recyclerview.widget.RecyclerView
import com.zinghr.app.api.TicketDto
import com.zinghr.app.databinding.ItemTicketBinding

class TicketAdapter(private val items: List<TicketDto>) :
    RecyclerView.Adapter<TicketAdapter.VH>() {

    inner class VH(val b: ItemTicketBinding) : RecyclerView.ViewHolder(b.root)

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int) =
        VH(ItemTicketBinding.inflate(LayoutInflater.from(parent.context), parent, false))

    override fun onBindViewHolder(holder: VH, position: Int) {
        val item = items[position]
        holder.b.tvTicketNo.text = item.ticketNo
        holder.b.tvSubject.text = item.subject
        holder.b.tvCategory.text = item.category.replaceFirstChar { it.uppercase() }
        holder.b.tvPriority.text = item.priority.replaceFirstChar { it.uppercase() }
        holder.b.tvStatus.text = item.status.replaceFirstChar { it.uppercase() }
        holder.b.tvDate.text = item.createdAt.take(10)
        val statusColor = when (item.status) {
            "resolved", "closed" -> "#4CAF50"
            "in_progress" -> "#2196F3"
            else -> "#FF9800"
        }
        holder.b.tvStatus.setTextColor(Color.parseColor(statusColor))
        val priorityColor = when (item.priority) {
            "urgent" -> "#F44336"
            "high" -> "#FF9800"
            "medium" -> "#2196F3"
            else -> "#9E9E9E"
        }
        holder.b.tvPriority.setTextColor(Color.parseColor(priorityColor))
    }

    override fun getItemCount() = items.size
}
