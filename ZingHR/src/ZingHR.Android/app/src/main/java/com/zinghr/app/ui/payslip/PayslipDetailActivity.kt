package com.zinghr.app.ui.payslip

import android.os.Bundle
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.zinghr.app.api.RetrofitClient
import com.zinghr.app.databinding.ActivityPayslipDetailBinding
import kotlinx.coroutines.launch
import java.text.NumberFormat
import java.util.Locale

class PayslipDetailActivity : AppCompatActivity() {

    private lateinit var binding: ActivityPayslipDetailBinding
    private val fmt = NumberFormat.getCurrencyInstance(Locale("en", "IN"))

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        binding = ActivityPayslipDetailBinding.inflate(layoutInflater)
        setContentView(binding.root)

        binding.toolbar.setNavigationOnClickListener { finish() }

        val payslipId = intent.getLongExtra("payslipId", 0)
        if (payslipId == 0L) { finish(); return }
        loadDetail(payslipId)
    }

    private fun loadDetail(id: Long) {
        lifecycleScope.launch {
            try {
                val api = RetrofitClient.getApi(this@PayslipDetailActivity)
                val response = api.getPayslipDetail(id)
                if (response.isSuccessful && response.body() != null) {
                    val p = response.body()!!
                    binding.tvPayPeriod.text = p.payPeriod
                    binding.tvEmpName.text = "${p.employeeName} (${p.empCode})"
                    binding.tvDeptDesig.text = "${p.designation} • ${p.department}"
                    binding.tvPaidDays.text = "Paid Days: ${p.paidDays} | LOP: ${p.lopDays}"

                    binding.tvBasic.text = fmt.format(p.basicSalary)
                    binding.tvHra.text = fmt.format(p.hra)
                    binding.tvConveyance.text = fmt.format(p.conveyanceAllowance)
                    binding.tvMedical.text = fmt.format(p.medicalAllowance)
                    binding.tvSpecial.text = fmt.format(p.specialAllowance)
                    binding.tvOtherEarnings.text = fmt.format(p.otherEarnings)
                    binding.tvGross.text = fmt.format(p.grossEarnings)

                    binding.tvPf.text = fmt.format(p.pf)
                    binding.tvEsi.text = fmt.format(p.esi)
                    binding.tvPt.text = fmt.format(p.professionalTax)
                    binding.tvTds.text = fmt.format(p.tds)
                    binding.tvLoan.text = fmt.format(p.loanDeduction)
                    binding.tvOtherDeductions.text = fmt.format(p.otherDeductions)
                    binding.tvTotalDeductions.text = fmt.format(p.totalDeductions)

                    binding.tvNetPay.text = fmt.format(p.netPay)
                }
            } catch (e: Exception) {
                Toast.makeText(this@PayslipDetailActivity, "Error: ${e.message}", Toast.LENGTH_SHORT).show()
            }
        }
    }
}
