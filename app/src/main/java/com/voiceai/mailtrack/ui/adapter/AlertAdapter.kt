package com.voiceai.mailtrack.ui.adapter

import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ImageButton
import android.widget.TextView
import androidx.recyclerview.widget.DiffUtil
import androidx.recyclerview.widget.ListAdapter
import androidx.recyclerview.widget.RecyclerView
import com.voiceai.mailtrack.R
import com.voiceai.mailtrack.data.entity.Alert
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

class AlertAdapter(
    private val onAcknowledge: (Alert) -> Unit
) : ListAdapter<Alert, AlertAdapter.AlertViewHolder>(AlertDiffCallback()) {

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): AlertViewHolder {
        val view = LayoutInflater.from(parent.context).inflate(R.layout.item_alert, parent, false)
        return AlertViewHolder(view)
    }

    override fun onBindViewHolder(holder: AlertViewHolder, position: Int) {
        holder.bind(getItem(position))
    }

    inner class AlertViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
        private val tvRuleName: TextView = itemView.findViewById(R.id.tv_rule_name)
        private val tvDescription: TextView = itemView.findViewById(R.id.tv_alert_description)
        private val tvSeverity: TextView = itemView.findViewById(R.id.tv_severity)
        private val tvTimestamp: TextView = itemView.findViewById(R.id.tv_alert_timestamp)
        private val btnAcknowledge: ImageButton = itemView.findViewById(R.id.btn_acknowledge)

        fun bind(alert: Alert) {
            tvRuleName.text = alert.ruleName
            tvDescription.text = alert.description
            tvSeverity.text = alert.severity.uppercase()
            tvTimestamp.text = SimpleDateFormat("MMM d, HH:mm", Locale.getDefault())
                .format(Date(alert.timestamp))

            val severityColor = when (alert.severity) {
                "critical" -> R.color.severity_critical
                "high" -> R.color.severity_high
                "medium" -> R.color.severity_medium
                else -> R.color.severity_low
            }
            tvSeverity.setBackgroundResource(severityColor)

            btnAcknowledge.visibility = if (alert.acknowledged) View.GONE else View.VISIBLE
            btnAcknowledge.setOnClickListener { onAcknowledge(alert) }
        }
    }

    class AlertDiffCallback : DiffUtil.ItemCallback<Alert>() {
        override fun areItemsTheSame(old: Alert, new: Alert) = old.id == new.id
        override fun areContentsTheSame(old: Alert, new: Alert) = old == new
    }
}
