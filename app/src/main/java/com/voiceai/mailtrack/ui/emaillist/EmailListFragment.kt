package com.voiceai.mailtrack.ui.emaillist

import android.content.Intent
import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.TextView
import androidx.appcompat.widget.SearchView
import androidx.fragment.app.Fragment
import androidx.fragment.app.activityViewModels
import androidx.recyclerview.widget.LinearLayoutManager
import androidx.recyclerview.widget.RecyclerView
import androidx.swiperefreshlayout.widget.SwipeRefreshLayout
import com.voiceai.mailtrack.R
import com.voiceai.mailtrack.ui.adapter.AlertAdapter
import com.voiceai.mailtrack.ui.adapter.EmailAdapter
import com.voiceai.mailtrack.ui.dashboard.DashboardViewModel
import com.voiceai.mailtrack.ui.emaildetail.EmailDetailActivity

class EmailListFragment : Fragment() {

    private val viewModel: DashboardViewModel by activityViewModels()
    private lateinit var emailAdapter: EmailAdapter
    private lateinit var alertAdapter: AlertAdapter
    private var filterType: String = "all"

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        filterType = arguments?.getString(ARG_FILTER) ?: "all"
    }

    override fun onCreateView(inflater: LayoutInflater, container: ViewGroup?, savedInstanceState: Bundle?): View {
        return inflater.inflate(R.layout.fragment_email_list, container, false)
    }

    override fun onViewCreated(view: View, savedInstanceState: Bundle?) {
        super.onViewCreated(view, savedInstanceState)

        val recyclerView = view.findViewById<RecyclerView>(R.id.recycler_view)
        val emptyView = view.findViewById<TextView>(R.id.tv_empty)
        val searchView = view.findViewById<SearchView>(R.id.search_view)
        val swipeRefresh = view.findViewById<SwipeRefreshLayout>(R.id.swipe_refresh)

        if (filterType == "alerts") {
            setupAlertsList(recyclerView, emptyView)
            searchView.visibility = View.GONE
        } else {
            setupEmailList(recyclerView, emptyView)
            setupSearch(searchView)
        }

        swipeRefresh.setOnRefreshListener { swipeRefresh.isRefreshing = false }
    }

    private fun setupEmailList(recyclerView: RecyclerView, emptyView: TextView) {
        emailAdapter = EmailAdapter(
            onItemClick = { email ->
                startActivity(Intent(requireContext(), EmailDetailActivity::class.java).apply {
                    putExtra("email_id", email.id)
                })
            },
            onFlagClick = { email -> viewModel.flagEmail(email) }
        )

        recyclerView.layoutManager = LinearLayoutManager(requireContext())
        recyclerView.adapter = emailAdapter

        val liveData = when (filterType) {
            "incoming" -> viewModel.getEmailsByDirection("incoming")
            "outgoing" -> viewModel.getEmailsByDirection("outgoing")
            else -> viewModel.allEmails
        }

        liveData.observe(viewLifecycleOwner) { emails ->
            emailAdapter.submitList(emails)
            emptyView.visibility = if (emails.isEmpty()) View.VISIBLE else View.GONE
            emptyView.text = "No ${filterType} emails tracked yet"
        }
    }

    private fun setupAlertsList(recyclerView: RecyclerView, emptyView: TextView) {
        alertAdapter = AlertAdapter { alert -> viewModel.acknowledgeAlert(alert.id) }

        recyclerView.layoutManager = LinearLayoutManager(requireContext())
        recyclerView.adapter = alertAdapter

        viewModel.unacknowledgedAlerts.observe(viewLifecycleOwner) { alerts ->
            alertAdapter.submitList(alerts)
            emptyView.visibility = if (alerts.isEmpty()) View.VISIBLE else View.GONE
            emptyView.text = "No active alerts"
        }
    }

    private fun setupSearch(searchView: SearchView) {
        searchView.setOnQueryTextListener(object : SearchView.OnQueryTextListener {
            override fun onQueryTextSubmit(query: String?): Boolean {
                query?.let { performSearch(it) }
                return true
            }

            override fun onQueryTextChange(newText: String?): Boolean {
                if (newText.isNullOrBlank()) {
                    observeDefault()
                } else if (newText.length >= 2) {
                    performSearch(newText)
                }
                return true
            }
        })
    }

    private fun performSearch(query: String) {
        viewModel.searchEmails(query).observe(viewLifecycleOwner) { results ->
            emailAdapter.submitList(results)
        }
    }

    private fun observeDefault() {
        val liveData = when (filterType) {
            "incoming" -> viewModel.getEmailsByDirection("incoming")
            "outgoing" -> viewModel.getEmailsByDirection("outgoing")
            else -> viewModel.allEmails
        }
        liveData.observe(viewLifecycleOwner) { emailAdapter.submitList(it) }
    }

    companion object {
        private const val ARG_FILTER = "filter"
        fun newInstance(filter: String) = EmailListFragment().apply {
            arguments = Bundle().apply { putString(ARG_FILTER, filter) }
        }
    }
}
