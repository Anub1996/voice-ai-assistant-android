package com.voiceai.mailtrack.data.db

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase
import com.voiceai.mailtrack.data.dao.AlertDao
import com.voiceai.mailtrack.data.dao.EmailDao
import com.voiceai.mailtrack.data.dao.RuleDao
import com.voiceai.mailtrack.data.entity.Alert
import com.voiceai.mailtrack.data.entity.DlpRule
import com.voiceai.mailtrack.data.entity.TrackedEmail

@Database(
    entities = [TrackedEmail::class, DlpRule::class, Alert::class],
    version = 1,
    exportSchema = false
)
abstract class MailTrackDatabase : RoomDatabase() {

    abstract fun emailDao(): EmailDao
    abstract fun ruleDao(): RuleDao
    abstract fun alertDao(): AlertDao

    companion object {
        @Volatile
        private var INSTANCE: MailTrackDatabase? = null

        fun getInstance(context: Context): MailTrackDatabase {
            return INSTANCE ?: synchronized(this) {
                INSTANCE ?: Room.databaseBuilder(
                    context.applicationContext,
                    MailTrackDatabase::class.java,
                    "mail_track.db"
                ).build().also { INSTANCE = it }
            }
        }
    }
}
