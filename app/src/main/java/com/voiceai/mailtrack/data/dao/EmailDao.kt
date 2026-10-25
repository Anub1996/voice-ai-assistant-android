package com.voiceai.mailtrack.data.dao

import androidx.lifecycle.LiveData
import androidx.room.*
import com.voiceai.mailtrack.data.entity.TrackedEmail

@Dao
interface EmailDao {

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insert(email: TrackedEmail): Long

    @Update
    suspend fun update(email: TrackedEmail)

    @Delete
    suspend fun delete(email: TrackedEmail)

    @Query("SELECT * FROM tracked_emails ORDER BY timestamp DESC")
    fun getAllEmails(): LiveData<List<TrackedEmail>>

    @Query("SELECT * FROM tracked_emails WHERE id = :id")
    suspend fun getById(id: Long): TrackedEmail?

    @Query("SELECT * FROM tracked_emails WHERE direction = :direction ORDER BY timestamp DESC")
    fun getByDirection(direction: String): LiveData<List<TrackedEmail>>

    @Query("SELECT * FROM tracked_emails WHERE flagged = 1 ORDER BY timestamp DESC")
    fun getFlaggedEmails(): LiveData<List<TrackedEmail>>

    @Query("SELECT * FROM tracked_emails WHERE alertTriggered = 1 ORDER BY timestamp DESC")
    fun getAlertedEmails(): LiveData<List<TrackedEmail>>

    @Query("SELECT * FROM tracked_emails WHERE sender LIKE '%' || :query || '%' OR recipients LIKE '%' || :query || '%' OR subject LIKE '%' || :query || '%' OR bodyPreview LIKE '%' || :query || '%' ORDER BY timestamp DESC")
    fun search(query: String): LiveData<List<TrackedEmail>>

    @Query("SELECT * FROM tracked_emails WHERE emailClient = :client ORDER BY timestamp DESC")
    fun getByClient(client: String): LiveData<List<TrackedEmail>>

    @Query("SELECT COUNT(*) FROM tracked_emails")
    fun getTotalCount(): LiveData<Int>

    @Query("SELECT COUNT(*) FROM tracked_emails WHERE direction = 'incoming'")
    fun getIncomingCount(): LiveData<Int>

    @Query("SELECT COUNT(*) FROM tracked_emails WHERE direction = 'outgoing'")
    fun getOutgoingCount(): LiveData<Int>

    @Query("SELECT COUNT(*) FROM tracked_emails WHERE alertTriggered = 1")
    fun getAlertCount(): LiveData<Int>

    @Query("SELECT COUNT(*) FROM tracked_emails WHERE timestamp >= :since")
    fun getCountSince(since: Long): LiveData<Int>

    @Query("SELECT emailClient, COUNT(*) as count FROM tracked_emails GROUP BY emailClient")
    fun getClientDistribution(): LiveData<List<ClientCount>>

    @Query("DELETE FROM tracked_emails WHERE timestamp < :before")
    suspend fun deleteOlderThan(before: Long)
}

data class ClientCount(val emailClient: String, val count: Int)
