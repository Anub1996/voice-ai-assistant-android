package com.olxclone.app.adapters

import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.recyclerview.widget.RecyclerView
import com.olxclone.app.R
import com.olxclone.app.api.Category

class CategoryAdapter(
    private val onClick: (Category) -> Unit
) : RecyclerView.Adapter<CategoryAdapter.VH>() {

    private val items = mutableListOf<Category>()

    fun updateData(data: List<Category>) {
        items.clear()
        items.addAll(data)
        notifyDataSetChanged()
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int) =
        VH(LayoutInflater.from(parent.context).inflate(R.layout.item_category, parent, false))

    override fun onBindViewHolder(holder: VH, position: Int) = holder.bind(items[position])
    override fun getItemCount() = items.size

    inner class VH(v: View) : RecyclerView.ViewHolder(v) {
        private val tvIcon: TextView = v.findViewById(R.id.tvIcon)
        private val tvName: TextView = v.findViewById(R.id.tvName)

        fun bind(item: Category) {
            tvIcon.text = item.icon
            tvName.text = item.name
            itemView.setOnClickListener { onClick(item) }
        }
    }
}
