package com.voiceai.mailtrack.ui.adapter

import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ImageButton
import android.widget.Switch
import android.widget.TextView
import androidx.recyclerview.widget.DiffUtil
import androidx.recyclerview.widget.ListAdapter
import androidx.recyclerview.widget.RecyclerView
import com.voiceai.mailtrack.R
import com.voiceai.mailtrack.data.entity.DlpRule

class RuleAdapter(
    private val onToggle: (DlpRule, Boolean) -> Unit,
    private val onEdit: (DlpRule) -> Unit,
    private val onDelete: (DlpRule) -> Unit
) : ListAdapter<DlpRule, RuleAdapter.RuleViewHolder>(RuleDiffCallback()) {

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): RuleViewHolder {
        val view = LayoutInflater.from(parent.context).inflate(R.layout.item_rule, parent, false)
        return RuleViewHolder(view)
    }

    override fun onBindViewHolder(holder: RuleViewHolder, position: Int) {
        holder.bind(getItem(position))
    }

    inner class RuleViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
        private val tvName: TextView = itemView.findViewById(R.id.tv_rule_name)
        private val tvDescription: TextView = itemView.findViewById(R.id.tv_rule_description)
        private val tvType: TextView = itemView.findViewById(R.id.tv_rule_type)
        private val tvSeverity: TextView = itemView.findViewById(R.id.tv_rule_severity)
        private val switchEnabled: Switch = itemView.findViewById(R.id.switch_enabled)
        private val btnEdit: ImageButton = itemView.findViewById(R.id.btn_edit)
        private val btnDelete: ImageButton = itemView.findViewById(R.id.btn_delete)

        fun bind(rule: DlpRule) {
            tvName.text = rule.name
            tvDescription.text = rule.description
            tvType.text = rule.ruleType.replace("_", " ").replaceFirstChar { it.uppercase() }
            tvSeverity.text = rule.severity.uppercase()

            switchEnabled.setOnCheckedChangeListener(null)
            switchEnabled.isChecked = rule.enabled
            switchEnabled.setOnCheckedChangeListener { _, checked -> onToggle(rule, checked) }

            btnEdit.setOnClickListener { onEdit(rule) }
            btnDelete.setOnClickListener { onDelete(rule) }
        }
    }

    class RuleDiffCallback : DiffUtil.ItemCallback<DlpRule>() {
        override fun areItemsTheSame(old: DlpRule, new: DlpRule) = old.id == new.id
        override fun areContentsTheSame(old: DlpRule, new: DlpRule) = old == new
    }
}
