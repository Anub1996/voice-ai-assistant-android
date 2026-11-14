package com.zinghr.app.ui.dashboard

import android.content.Intent
import android.os.Bundle
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.zinghr.app.api.RetrofitClient
import com.zinghr.app.databinding.ActivityMainBinding
import com.zinghr.app.ui.attendance.AttendanceActivity
import com.zinghr.app.ui.auth.LoginActivity
import com.zinghr.app.ui.directory.DirectoryActivity
import com.zinghr.app.ui.expense.ExpenseActivity
import com.zinghr.app.ui.helpdesk.HelpdeskActivity
import com.zinghr.app.ui.leave.LeaveActivity
import com.zinghr.app.ui.notifications.NotificationsActivity
import com.zinghr.app.ui.payslip.PayslipActivity
import com.zinghr.app.ui.profile.ProfileActivity
import com.zinghr.app.utils.SessionManager
import kotlinx.coroutines.launch

class MainActivity : AppCompatActivity() {

    private lateinit var binding: ActivityMainBinding
    private lateinit var session: SessionManager

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityMainBinding.inflate(layoutInflater)
        setContentView(binding.root)

        session = SessionManager(this)
        binding.tvGreeting.text = "Hello, ${session.getFullName()}"
        binding.tvRole.text = session.getRole().replaceFirstChar { it.uppercase() }

        setupMenuItems()
        loadDashboard()
    }

    override fun onResume() {
        super.onResume()
        loadNotificationBadge()
    }

    private fun setupMenuItems() {
        binding.cardAttendance.setOnClickListener { startActivity(Intent(this, AttendanceActivity::class.java)) }
        binding.cardLeave.setOnClickListener { startActivity(Intent(this, LeaveActivity::class.java)) }
        binding.cardPayslip.setOnClickListener { startActivity(Intent(this, PayslipActivity::class.java)) }
        binding.cardExpense.setOnClickListener { startActivity(Intent(this, ExpenseActivity::class.java)) }
        binding.cardDirectory.setOnClickListener { startActivity(Intent(this, DirectoryActivity::class.java)) }
        binding.cardHelpdesk.setOnClickListener { startActivity(Intent(this, HelpdeskActivity::class.java)) }
        binding.cardProfile.setOnClickListener { startActivity(Intent(this, ProfileActivity::class.java)) }
        binding.cardNotifications.setOnClickListener { startActivity(Intent(this, NotificationsActivity::class.java)) }

        binding.btnLogout.setOnClickListener {
            session.logout()
            startActivity(Intent(this, LoginActivity::class.java))
            finish()
        }
    }

    private fun loadDashboard() {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@MainActivity)
                val response = api.getEmployeeDashboard()
                if (response.isSuccessful && response.body() != null) {
                    val data = response.body()!!
                    binding.tvDesignation.text = "${data.designation} • ${data.department}"

                    if (data.todayAttendance != null) {
                        val att = data.todayAttendance
                        binding.tvAttendanceStatus.text = when {
                            att.punchOut != null -> "Punched Out • ${att.workingMinutes / 60}h ${att.workingMinutes % 60}m"
                            att.punchIn != null -> "Working..."
                            else -> "Not Punched In"
                        }
                        binding.tvAttendanceStatus.visibility = View.VISIBLE
                    }

                    binding.tvPendingLeaves.text = "${data.pendingLeaves}"
                    binding.tvPendingExpenses.text = "${data.pendingExpenses}"
                    binding.tvOpenTickets.text = "${data.openTickets}"

                    if (data.leaveBalance.isNotEmpty()) {
                        val cl = data.leaveBalance.firstOrNull { it.leaveTypeName.contains("Casual", true) }
                        val sl = data.leaveBalance.firstOrNull { it.leaveTypeName.contains("Sick", true) }
                        binding.tvLeaveBalance.text = "CL: ${cl?.balance?.toInt() ?: 0} | SL: ${sl?.balance?.toInt() ?: 0}"
                        binding.tvLeaveBalance.visibility = View.VISIBLE
                    }

                    if (data.recentAnnouncements.isNotEmpty()) {
                        binding.tvAnnouncement.text = data.recentAnnouncements.first().title
                        binding.cardAnnouncement.visibility = View.VISIBLE
                    }
                }
            } catch (e: Exception) {
                Toast.makeText(this@MainActivity, "Error loading dashboard", Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun loadNotificationBadge() {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@MainActivity)
                val response = api.getUnreadCount()
                if (response.isSuccessful && response.body() != null) {
                    val count = response.body()!!.count
                    binding.tvNotifBadge.visibility = if (count > 0) View.VISIBLE else View.GONE
                    binding.tvNotifBadge.text = if (count > 9) "9+" else count.toString()
                }
            } catch (_: Exception) { }
        }
    }
}
