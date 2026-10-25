package com.voiceai.mailtrack.ui.dashboard

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.LiveData
import androidx.lifecycle.MutableLiveData
import androidx.lifecycle.viewModelScope
import com.voiceai.mailtrack.MailTrackApp
import com.voiceai.mailtrack.data.entity.Alert
import com.voiceai.mailtrack.data.entity.TrackedEmail
import com.voiceai.mailtrack.data.repository.MailTrackRepository
import kotlinx.coroutines.launch

class DashboardViewModel(application: Application) : AndroidViewModel(application) {

    private val repository: MailTrackRepository

    val allEmails: LiveData<List<TrackedEmail>>
    val totalCount: LiveData<Int>
    val incomingCount: LiveData<Int>
    val outgoingCount: LiveData<Int>
    val alertCount: LiveData<Int>
    val activeRuleCount: LiveData<Int>
    val unacknowledgedAlerts: LiveData<List<Alert>>
    val unacknowledgedAlertCount: LiveData<Int>

    private val _searchResults = MutableLiveData<List<TrackedEmail>>()
    val searchResults: LiveData<List<TrackedEmail>> = _searchResults

    private val _currentFilter = MutableLiveData("all")
    val currentFilter: LiveData<String> = _currentFilter

    init {
        val db = (application as MailTrackApp).database
        repository = MailTrackRepository(db.emailDao(), db.ruleDao(), db.alertDao())

        allEmails = repository.allEmails
        totalCount = repository.totalEmailCount
        incomingCount = repository.incomingCount
        outgoingCount = repository.outgoingCount
        alertCount = repository.alertCount
        activeRuleCount = repository.activeRuleCount
        unacknowledgedAlerts = repository.unacknowledgedAlerts
        unacknowledgedAlertCount = repository.unacknowledgedAlertCount
    }

    fun searchEmails(query: String): LiveData<List<TrackedEmail>> {
        return repository.searchEmails(query)
    }

    fun getEmailsByDirection(direction: String): LiveData<List<TrackedEmail>> {
        return repository.getEmailsByDirection(direction)
    }

    fun setFilter(filter: String) {
        _currentFilter.value = filter
    }

    fun deleteEmail(email: TrackedEmail) {
        viewModelScope.launch { repository.deleteEmail(email) }
    }

    fun flagEmail(email: TrackedEmail) {
        viewModelScope.launch { repository.updateEmail(email.copy(flagged = !email.flagged)) }
    }

    fun acknowledgeAlert(alertId: Long) {
        viewModelScope.launch { repository.acknowledgeAlert(alertId) }
    }

    fun acknowledgeAllAlerts() {
        viewModelScope.launch { repository.acknowledgeAllAlerts() }
    }
}
