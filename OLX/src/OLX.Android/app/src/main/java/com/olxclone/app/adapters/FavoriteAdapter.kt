package com.olxclone.app.adapters

import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ImageButton
import android.widget.ImageView
import android.widget.TextView
import androidx.recyclerview.widget.RecyclerView
import com.olxclone.app.R
import com.olxclone.app.api.FavoriteItem
import java.text.NumberFormat
import java.util.Locale

class FavoriteAdapter(
    private val onClick: (FavoriteItem) -> Unit,
    private val onRemove: (FavoriteItem) -> Unit
) : RecyclerView.Adapter<FavoriteAdapter.VH>() {

    private val items = mutableListOf<FavoriteItem>()

    fun updateData(data: List<FavoriteItem>) {
        items.clear()
        items.addAll(data)
        notifyDataSetChanged()
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int) =
        VH(LayoutInflater.from(parent.context).inflate(R.layout.item_favorite, parent, false))

    override fun onBindViewHolder(holder: VH, position: Int) = holder.bind(items[position])
    override fun getItemCount() = items.size

    inner class VH(v: View) : RecyclerView.ViewHolder(v) {
        private val ivThumb: ImageView = v.findViewById(R.id.ivThumb)
        private val tvPrice: TextView = v.findViewById(R.id.tvPrice)
        private val tvTitle: TextView = v.findViewById(R.id.tvTitle)
        private val tvCity: TextView = v.findViewById(R.id.tvCity)
        private val btnRemove: ImageButton = v.findViewById(R.id.btnRemove)

        fun bind(item: FavoriteItem) {
            val fmt = NumberFormat.getCurrencyInstance(Locale("en", "IN"))
            fmt.maximumFractionDigits = 0
            tvPrice.text = fmt.format(item.price)
            tvTitle.text = item.title
            tvCity.text = item.city ?: ""
            btnRemove.setOnClickListener { onRemove(item) }
            itemView.setOnClickListener { onClick(item) }
        }
    }
}
