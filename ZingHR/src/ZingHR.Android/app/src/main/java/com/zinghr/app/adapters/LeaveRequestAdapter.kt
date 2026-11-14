package com.zinghr.app.adapters

import android.graphics.Color
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import androidx.recyclerview.widget.RecyclerView
import com.zinghr.app.api.LeaveRequestDto
import com.zinghr.app.databinding.ItemLeaveRequestBinding

class LeaveRequestAdapter(
    private val items: List<LeaveRequestDto>,
    private val onCancel: (Long) -> Unit
) : RecyclerView.Adapter<LeaveRequestAdapter.VH>() {

    inner class VH(val b: ItemLeaveRequestBinding) : RecyclerView.ViewHolder(b.root)

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int) =
        VH(ItemLeaveRequestBinding.inflate(LayoutInflater.from(parent.context), parent, false))

    override fun onBindViewHolder(holder: VH, position: Int) {
        val item = items[position]
        holder.b.tvLeaveType.text = item.leaveType
        try { holder.b.colorIndicator.setBackgroundColor(Color.parseColor(item.leaveTypeColor)) } catch (_: Exception) {}
        holder.b.tvDates.text = "${item.fromDate} - ${item.toDate}"
        holder.b.tvDays.text = "${item.totalDays} day(s)"
        holder.b.tvReason.text = item.reason
        holder.b.tvStatus.text = item.status.replaceFirstChar { it.uppercase() }
        val statusColor = when (item.status) {
            "approved" -> "#4CAF50"
            "rejected" -> "#F44336"
            else -> "#FF9800"
        }
        holder.b.tvStatus.setTextColor(Color.parseColor(statusColor))
        if (item.status == "pending") {
            holder.b.btnCancel.visibility = View.VISIBLE
            holder.b.btnCancel.setOnClickListener { onCancel(item.id) }
        } else {
            holder.b.btnCancel.visibility = View.GONE
        }
    }

    override fun getItemCount() = items.size
}
