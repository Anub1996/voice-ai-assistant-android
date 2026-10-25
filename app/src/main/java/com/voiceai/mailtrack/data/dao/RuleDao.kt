package com.voiceai.mailtrack.data.dao

import androidx.lifecycle.LiveData
import androidx.room.*
import com.voiceai.mailtrack.data.entity.DlpRule

@Dao
interface RuleDao {

    @Insert(onConflict = OnConflictStrategy.REPLACE)
    suspend fun insert(rule: DlpRule): Long

    @Update
    suspend fun update(rule: DlpRule)

    @Delete
    suspend fun delete(rule: DlpRule)

    @Query("SELECT * FROM dlp_rules ORDER BY createdAt DESC")
    fun getAllRules(): LiveData<List<DlpRule>>

    @Query("SELECT * FROM dlp_rules WHERE enabled = 1")
    suspend fun getEnabledRules(): List<DlpRule>

    @Query("SELECT * FROM dlp_rules WHERE id = :id")
    suspend fun getById(id: Long): DlpRule?

    @Query("SELECT COUNT(*) FROM dlp_rules WHERE enabled = 1")
    fun getActiveRuleCount(): LiveData<Int>
}
