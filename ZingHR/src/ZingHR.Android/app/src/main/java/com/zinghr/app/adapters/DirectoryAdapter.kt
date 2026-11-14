package com.zinghr.app.adapters

import android.view.LayoutInflater
import android.view.ViewGroup
import androidx.recyclerview.widget.RecyclerView
import com.zinghr.app.api.DirectoryEntryDto
import com.zinghr.app.databinding.ItemDirectoryBinding

class DirectoryAdapter(private val items: List<DirectoryEntryDto>) :
    RecyclerView.Adapter<DirectoryAdapter.VH>() {

    inner class VH(val b: ItemDirectoryBinding) : RecyclerView.ViewHolder(b.root)

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int) =
        VH(ItemDirectoryBinding.inflate(LayoutInflater.from(parent.context), parent, false))

    override fun onBindViewHolder(holder: VH, position: Int) {
        val item = items[position]
        holder.b.tvName.text = item.fullName
        holder.b.tvEmpCode.text = item.empCode
        holder.b.tvDepartment.text = item.department
        holder.b.tvDesignation.text = item.designation
        holder.b.tvEmail.text = item.email
        holder.b.tvPhone.text = item.phone.ifEmpty { "N/A" }
        holder.b.tvInitials.text = item.fullName.split(" ").take(2).joinToString("") { it.first().uppercase() }
    }

    override fun getItemCount() = items.size
}
