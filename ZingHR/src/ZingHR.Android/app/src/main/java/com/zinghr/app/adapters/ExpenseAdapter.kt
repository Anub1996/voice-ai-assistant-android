package com.zinghr.app.adapters

import android.graphics.Color
import android.view.LayoutInflater
import android.view.ViewGroup
import androidx.recyclerview.widget.RecyclerView
import com.zinghr.app.api.ExpenseDto
import com.zinghr.app.databinding.ItemExpenseBinding
import java.text.NumberFormat
import java.util.Locale

class ExpenseAdapter(private val items: List<ExpenseDto>) :
    RecyclerView.Adapter<ExpenseAdapter.VH>() {

    private val fmt = NumberFormat.getCurrencyInstance(Locale("en", "IN"))

    inner class VH(val b: ItemExpenseBinding) : RecyclerView.ViewHolder(b.root)

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int) =
        VH(ItemExpenseBinding.inflate(LayoutInflater.from(parent.context), parent, false))

    override fun onBindViewHolder(holder: VH, position: Int) {
        val item = items[position]
        holder.b.tvTitle.text = item.title
        holder.b.tvCategory.text = item.category
        holder.b.tvAmount.text = fmt.format(item.amount)
        holder.b.tvDate.text = item.expenseDate
        holder.b.tvStatus.text = item.status.replaceFirstChar { it.uppercase() }
        val statusColor = when (item.status) {
            "approved" -> "#4CAF50"
            "rejected" -> "#F44336"
            else -> "#FF9800"
        }
        holder.b.tvStatus.setTextColor(Color.parseColor(statusColor))
    }

    override fun getItemCount() = items.size
}
