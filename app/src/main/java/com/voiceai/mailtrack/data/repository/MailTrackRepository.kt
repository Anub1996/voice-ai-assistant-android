package com.voiceai.mailtrack.data.repository

import androidx.lifecycle.LiveData
import com.voiceai.mailtrack.data.dao.AlertDao
import com.voiceai.mailtrack.data.dao.EmailDao
import com.voiceai.mailtrack.data.dao.RuleDao
import com.voiceai.mailtrack.data.entity.Alert
import com.voiceai.mailtrack.data.entity.DlpRule
import com.voiceai.mailtrack.data.entity.TrackedEmail

class MailTrackRepository(
    private val emailDao: EmailDao,
    private val ruleDao: RuleDao,
    private val alertDao: AlertDao
) {
    val allEmails: LiveData<List<TrackedEmail>> = emailDao.getAllEmails()
    val flaggedEmails: LiveData<List<TrackedEmail>> = emailDao.getFlaggedEmails()
    val alertedEmails: LiveData<List<TrackedEmail>> = emailDao.getAlertedEmails()
    val allRules: LiveData<List<DlpRule>> = ruleDao.getAllRules()
    val allAlerts: LiveData<List<Alert>> = alertDao.getAllAlerts()
    val unacknowledgedAlerts: LiveData<List<Alert>> = alertDao.getUnacknowledgedAlerts()
    val totalEmailCount: LiveData<Int> = emailDao.getTotalCount()
    val incomingCount: LiveData<Int> = emailDao.getIncomingCount()
    val outgoingCount: LiveData<Int> = emailDao.getOutgoingCount()
    val alertCount: LiveData<Int> = emailDao.getAlertCount()
    val activeRuleCount: LiveData<Int> = ruleDao.getActiveRuleCount()
    val unacknowledgedAlertCount: LiveData<Int> = alertDao.getUnacknowledgedCount()

    suspend fun insertEmail(email: TrackedEmail): Long = emailDao.insert(email)
    suspend fun updateEmail(email: TrackedEmail) = emailDao.update(email)
    suspend fun deleteEmail(email: TrackedEmail) = emailDao.delete(email)
    suspend fun getEmailById(id: Long): TrackedEmail? = emailDao.getById(id)
    fun searchEmails(query: String): LiveData<List<TrackedEmail>> = emailDao.search(query)
    fun getEmailsByDirection(direction: String) = emailDao.getByDirection(direction)
    fun getEmailsByClient(client: String) = emailDao.getByClient(client)
    fun getEmailCountSince(since: Long) = emailDao.getCountSince(since)

    suspend fun insertRule(rule: DlpRule): Long = ruleDao.insert(rule)
    suspend fun updateRule(rule: DlpRule) = ruleDao.update(rule)
    suspend fun deleteRule(rule: DlpRule) = ruleDao.delete(rule)
    suspend fun getEnabledRules(): List<DlpRule> = ruleDao.getEnabledRules()
    suspend fun getRuleById(id: Long): DlpRule? = ruleDao.getById(id)

    suspend fun insertAlert(alert: Alert): Long = alertDao.insert(alert)
    suspend fun acknowledgeAlert(id: Long) = alertDao.acknowledge(id)
    suspend fun acknowledgeAllAlerts() = alertDao.acknowledgeAll()
    suspend fun getAlertsByEmailId(emailId: Long): List<Alert> = alertDao.getByEmailId(emailId)
}
