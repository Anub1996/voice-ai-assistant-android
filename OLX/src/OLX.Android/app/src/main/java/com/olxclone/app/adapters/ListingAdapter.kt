package com.olxclone.app.adapters

import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.TextView
import androidx.core.view.isVisible
import androidx.recyclerview.widget.RecyclerView
import com.olxclone.app.R
import com.olxclone.app.api.ListingItem
import java.text.NumberFormat
import java.util.Locale

class ListingAdapter(
    private val onClick: (ListingItem) -> Unit,
    private val onFavClick: ((ListingItem) -> Unit)? = null
) : RecyclerView.Adapter<ListingAdapter.VH>() {

    private val items = mutableListOf<ListingItem>()

    fun updateData(data: List<ListingItem>) {
        items.clear()
        items.addAll(data)
        notifyDataSetChanged()
    }

    fun addData(data: List<ListingItem>) {
        val start = items.size
        items.addAll(data)
        notifyItemRangeInserted(start, data.size)
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int) =
        VH(LayoutInflater.from(parent.context).inflate(R.layout.item_listing, parent, false))

    override fun onBindViewHolder(holder: VH, position: Int) = holder.bind(items[position])
    override fun getItemCount() = items.size

    inner class VH(v: View) : RecyclerView.ViewHolder(v) {
        private val ivThumb: ImageView = v.findViewById(R.id.ivThumb)
        private val tvPrice: TextView = v.findViewById(R.id.tvPrice)
        private val tvTitle: TextView = v.findViewById(R.id.tvTitle)
        private val tvLocation: TextView = v.findViewById(R.id.tvLocation)
        private val tvFeatured: TextView = v.findViewById(R.id.tvFeatured)
        private val ivFav: ImageView = v.findViewById(R.id.ivFav)

        fun bind(item: ListingItem) {
            val fmt = NumberFormat.getCurrencyInstance(Locale("en", "IN"))
            fmt.maximumFractionDigits = 0
            tvPrice.text = fmt.format(item.price)
            tvTitle.text = item.title
            tvLocation.text = listOfNotNull(item.city, item.state).joinToString(", ")
            tvFeatured.isVisible = item.isFeatured
            ivFav.setImageResource(
                if (item.isFavorited) android.R.drawable.btn_star_big_on
                else android.R.drawable.btn_star_big_off
            )
            ivFav.setOnClickListener { onFavClick?.invoke(item) }
            itemView.setOnClickListener { onClick(item) }
        }
    }
}
