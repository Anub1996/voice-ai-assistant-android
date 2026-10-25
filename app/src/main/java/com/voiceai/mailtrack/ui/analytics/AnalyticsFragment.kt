package com.voiceai.mailtrack.ui.analytics

import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.fragment.app.Fragment
import androidx.fragment.app.activityViewModels
import com.voiceai.mailtrack.R
import com.voiceai.mailtrack.ui.dashboard.DashboardViewModel

class AnalyticsFragment : Fragment() {

    private val viewModel: DashboardViewModel by activityViewModels()

    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, savedInstanceState: Bundle?): View {
        return inflater.inflate(R.layout.fragment_analytics, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)

        val tvTotalEmails = view.findViewById<TextView>(R.id.tv_stat_total)
        val tvIncoming = view.findViewById<TextView>(R.id.tv_stat_incoming)
        val tvOutgoing = view.findViewById<TextView>(R.id.tv_stat_outgoing)
        val tvAlerts = view.findViewById<TextView>(R.id.tv_stat_alerts)
        val tvActiveRules = view.findViewById<TextView>(R.id.tv_stat_active_rules)
        val tvPendingAlerts = view.findViewById<TextView>(R.id.tv_stat_pending_alerts)

        viewModel.totalCount.observe(viewLifecycleOwner) { tvTotalEmails.text = it.toString() }
        viewModel.incomingCount.observe(viewLifecycleOwner) { tvIncoming.text = it.toString() }
        viewModel.outgoingCount.observe(viewLifecycleOwner) { tvOutgoing.text = it.toString() }
        viewModel.alertCount.observe(viewLifecycleOwner) { tvAlerts.text = it.toString() }
        viewModel.activeRuleCount.observe(viewLifecycleOwner) { tvActiveRules.text = it.toString() }
        viewModel.unacknowledgedAlertCount.observe(viewLifecycleOwner) { tvPendingAlerts.text = it.toString() }
    }
}
