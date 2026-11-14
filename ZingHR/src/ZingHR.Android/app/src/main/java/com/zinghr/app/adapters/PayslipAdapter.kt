package com.zinghr.app.adapters

import android.view.LayoutInflater
import android.view.ViewGroup
import androidx.recyclerview.widget.RecyclerView
import com.zinghr.app.api.PayslipListDto
import com.zinghr.app.databinding.ItemPayslipBinding
import java.text.NumberFormat
import java.util.Locale

class PayslipAdapter(
    private val items: List<PayslipListDto>,
    private val onClick: (Long) -> Unit
) : RecyclerView.Adapter<PayslipAdapter.VH>() {

    private val fmt = NumberFormat.getCurrencyInstance(Locale("en", "IN"))

    inner class VH(val b: ItemPayslipBinding) : RecyclerView.ViewHolder(b.root)

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int) =
        VH(ItemPayslipBinding.inflate(LayoutInflater.from(parent.context), parent, false))

    override fun onBindViewHolder(holder: VH, position: Int) {
        val item = items[position]
        holder.b.tvPayPeriod.text = item.payPeriod
        holder.b.tvNetPay.text = fmt.format(item.netPay)
        holder.b.tvStatus.text = item.status.replaceFirstChar { it.uppercase() }
        holder.itemView.setOnClickListener { onClick(item.id) }
    }

    override fun getItemCount() = items.size
}
