package com.voiceai.mailtrack.ui.adapter

import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.ImageView
import android.widget.TextView
import androidx.recyclerview.widget.DiffUtil
import androidx.recyclerview.widget.ListAdapter
import androidx.recyclerview.widget.RecyclerView
import com.voiceai.mailtrack.R
import com.voiceai.mailtrack.data.entity.TrackedEmail
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

class EmailAdapter(
    private val onItemClick: (TrackedEmail) -> Unit,
    private val onFlagClick: (TrackedEmail) -> Unit
) : ListAdapter<TrackedEmail, EmailAdapter.EmailViewHolder>(EmailDiffCallback()) {

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): EmailViewHolder {
        val view = LayoutInflater.from(parent.context).inflate(R.layout.item_email, parent, false)
        return EmailViewHolder(view)
    }

    override fun onBindViewHolder(holder: EmailViewHolder, position: Int) {
        holder.bind(getItem(position))
    }

    inner class EmailViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
        private val tvSender: TextView = itemView.findViewById(R.id.tv_sender)
        private val tvSubject: TextView = itemView.findViewById(R.id.tv_subject)
        private val tvPreview: TextView = itemView.findViewById(R.id.tv_preview)
        private val tvTimestamp: TextView = itemView.findViewById(R.id.tv_timestamp)
        private val tvClient: TextView = itemView.findViewById(R.id.tv_client)
        private val ivDirection: ImageView = itemView.findViewById(R.id.iv_direction)
        private val ivFlag: ImageView = itemView.findViewById(R.id.iv_flag)
        private val ivAlert: ImageView = itemView.findViewById(R.id.iv_alert)
        private val ivAttachment: ImageView = itemView.findViewById(R.id.iv_attachment)

        fun bind(email: TrackedEmail) {
            tvSender.text = email.sender.ifBlank { "(unknown)" }
            tvSubject.text = email.subject.ifBlank { "(no subject)" }
            tvPreview.text = email.bodyPreview.ifBlank { "" }
            tvTimestamp.text = formatTimestamp(email.timestamp)
            tvClient.text = email.emailClient

            ivDirection.setImageResource(
                if (email.direction == "incoming") R.drawable.ic_incoming else R.drawable.ic_outgoing
            )
            ivFlag.visibility = if (email.flagged) View.VISIBLE else View.GONE
            ivAlert.visibility = if (email.alertTriggered) View.VISIBLE else View.GONE
            ivAttachment.visibility = if (email.hasAttachment) View.VISIBLE else View.GONE

            itemView.setOnClickListener { onItemClick(email) }
            ivFlag.setOnClickListener { onFlagClick(email) }
            itemView.setOnLongClickListener {
                onFlagClick(email)
                true
            }
        }

        private fun formatTimestamp(timestamp: Long): String {
            val now = System.currentTimeMillis()
            val diff = now - timestamp
            return when {
                diff < 60_000 -> "Just now"
                diff < 3_600_000 -> "${diff / 60_000}m ago"
                diff < 86_400_000 -> "${diff / 3_600_000}h ago"
                else -> SimpleDateFormat("MMM d, HH:mm", Locale.getDefault()).format(Date(timestamp))
            }
        }
    }

    class EmailDiffCallback : DiffUtil.ItemCallback<TrackedEmail>() {
        override fun areItemsTheSame(old: TrackedEmail, new: TrackedEmail) = old.id == new.id
        override fun areContentsTheSame(old: TrackedEmail, new: TrackedEmail) = old == new
    }
}
