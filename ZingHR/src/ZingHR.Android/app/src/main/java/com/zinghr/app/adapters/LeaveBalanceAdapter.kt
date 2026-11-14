package com.zinghr.app.adapters

import android.graphics.Color
import android.view.LayoutInflater
import android.view.ViewGroup
import androidx.recyclerview.widget.RecyclerView
import com.zinghr.app.api.LeaveBalanceDto
import com.zinghr.app.databinding.ItemLeaveBalanceBinding

class LeaveBalanceAdapter(private val items: List<LeaveBalanceDto>) :
    RecyclerView.Adapter<LeaveBalanceAdapter.VH>() {

    inner class VH(val b: ItemLeaveBalanceBinding) : RecyclerView.ViewHolder(b.root)

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int) =
        VH(ItemLeaveBalanceBinding.inflate(LayoutInflater.from(parent.context), parent, false))

    override fun onBindViewHolder(holder: VH, position: Int) {
        val item = items[position]
        holder.b.tvLeaveType.text = item.leaveTypeName
        holder.b.tvBalance.text = "${item.balance.toInt()}"
        holder.b.tvEntitled.text = "of ${item.entitled.toInt()}"
        try { holder.b.colorIndicator.setBackgroundColor(Color.parseColor(item.color)) } catch (_: Exception) { }
    }

    override fun getItemCount() = items.size
}
