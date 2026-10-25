package com.voiceai.mailtrack.data.dao

import androidx.lifecycle.LiveData
import androidx.room.*
import com.voiceai.mailtrack.data.entity.Alert

@Dao
interface AlertDao {

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insert(alert: Alert): Long

    @Update
    suspend fun update(alert: Alert)

    @Query("SELECT * FROM alerts ORDER BY timestamp DESC")
    fun getAllAlerts(): LiveData<List<Alert>>

    @Query("SELECT * FROM alerts WHERE acknowledged = 0 ORDER BY timestamp DESC")
    fun getUnacknowledgedAlerts(): LiveData<List<Alert>>

    @Query("SELECT * FROM alerts WHERE emailId = :emailId")
    suspend fun getByEmailId(emailId: Long): List<Alert>

    @Query("SELECT COUNT(*) FROM alerts WHERE acknowledged = 0")
    fun getUnacknowledgedCount(): LiveData<Int>

    @Query("UPDATE alerts SET acknowledged = 1 WHERE id = :id")
    suspend fun acknowledge(id: Long)

    @Query("UPDATE alerts SET acknowledged = 1")
    suspend fun acknowledgeAll()
}
