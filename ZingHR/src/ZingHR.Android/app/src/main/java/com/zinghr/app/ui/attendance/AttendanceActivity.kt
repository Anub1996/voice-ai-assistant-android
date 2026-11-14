package com.zinghr.app.ui.attendance

import android.Manifest
import android.content.pm.PackageManager
import android.os.Bundle
import android.view.View
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import androidx.lifecycle.lifecycleScope
import com.google.android.gms.location.LocationServices
import com.zinghr.app.api.PunchRequest
import com.zinghr.app.api.RetrofitClient
import com.zinghr.app.databinding.ActivityAttendanceBinding
import kotlinx.coroutines.launch

class AttendanceActivity : AppCompatActivity() {

    private lateinit var binding: ActivityAttendanceBinding
    private var currentLat = 0.0
    private var currentLng = 0.0

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityAttendanceBinding.inflate(layoutInflater)
        setContentView(binding.root)

        binding.toolbar.setNavigationOnClickListener { finish() }
        binding.btnPunchIn.setOnClickListener { punch(true) }
        binding.btnPunchOut.setOnClickListener { punch(false) }

        getCurrentLocation()
        loadTodayAttendance()
        loadSummary()
    }

    private fun getCurrentLocation() {
        if (ActivityCompat.checkSelfPermission(this, Manifest.permission.ACCESS_FINE_LOCATION) != PackageManager.PERMISSION_GRANTED) {
            ActivityCompat.requestPermissions(this, arrayOf(Manifest.permission.ACCESS_FINE_LOCATION), 100)
            return
        }
        val client = LocationServices.getFusedLocationProviderClient(this)
        client.lastLocation.addOnSuccessListener { loc ->
            if (loc != null) {
                currentLat = loc.latitude
                currentLng = loc.longitude
                binding.tvLocation.text = "📍 ${String.format("%.4f", loc.latitude)}, ${String.format("%.4f", loc.longitude)}"
            }
        }
    }

    private fun punch(isPunchIn: Boolean) {
        binding.progressBar.visibility = View.VISIBLE

        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@AttendanceActivity)
                val req = PunchRequest(currentLat, currentLng, null)
                val response = if (isPunchIn) api.punchIn(req) else api.punchOut(req)

                if (response.isSuccessful && response.body() != null) {
                    Toast.makeText(this@AttendanceActivity, response.body()!!.message, Toast.LENGTH_SHORT).show()
                    loadTodayAttendance()
                } else {
                    val error = response.errorBody()?.string() ?: "Failed"
                    Toast.makeText(this@AttendanceActivity, error, Toast.LENGTH_SHORT).show()
                }
            } catch (e: Exception) {
                Toast.makeText(this@AttendanceActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            } finally {
                binding.progressBar.visibility = View.GONE
            }
        }
    }

    private fun loadTodayAttendance() {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@AttendanceActivity)
                val response = api.getAttendanceToday()
                if (response.isSuccessful && response.body() != null) {
                    val att = response.body()!!
                    binding.tvStatus.text = att.status.replaceFirstChar { it.uppercase() }
                    binding.tvPunchIn.text = if (att.punchIn != null) "In: ${att.punchIn.substringAfter("T").take(5)}" else "Not punched in"
                    binding.tvPunchOut.text = if (att.punchOut != null) "Out: ${att.punchOut.substringAfter("T").take(5)}" else "--"
                    binding.tvWorkingHours.text = "${att.workingMinutes / 60}h ${att.workingMinutes % 60}m"

                    binding.btnPunchIn.isEnabled = att.punchIn == null
                    binding.btnPunchOut.isEnabled = att.punchIn != null && att.punchOut == null

                    if (att.lateByMinutes > 0)
                        binding.tvLateBy.text = "Late by ${att.lateByMinutes} min"
                }
            } catch (_: Exception) { }
        }
    }

    private fun loadSummary() {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@AttendanceActivity)
                val now = java.util.Calendar.getInstance()
                val response = api.getAttendanceSummary(now.get(java.util.Calendar.MONTH) + 1, now.get(java.util.Calendar.YEAR))
                if (response.isSuccessful && response.body() != null) {
                    val s = response.body()!!
                    binding.tvPresent.text = "${s.presentDays}"
                    binding.tvAbsent.text = "${s.absentDays}"
                    binding.tvLeave.text = "${s.leaveDays}"
                    binding.tvAvgHours.text = "${s.avgWorkingHours}h"
                }
            } catch (_: Exception) { }
        }
    }

    override fun onRequestPermissionsResult(requestCode: Int, permissions: Array<out String>, grantResults: IntArray) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == 100 && grantResults.isNotEmpty() && grantResults[0] == PackageManager.PERMISSION_GRANTED) {
            getCurrentLocation()
        }
    }
}
