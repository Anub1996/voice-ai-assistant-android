package com.zinghr.app.api

import com.google.gson.annotations.SerializedName

// ── Auth ──
data class LoginRequest(val username: String, val password: String)
data class PinLoginRequest(val username: String, val pin: String)
data class LoginResponse(
    val token: String,
    val username: String,
    val fullName: String,
    val role: String,
    val employeeId: Int?,
    val expiresAt: String
)
data class SetPinRequest(val pin: String)
data class ChangePasswordRequest(val oldPassword: String, val newPassword: String)
data class DeviceTokenRequest(val token: String)

// ── Profile ──
data class ProfileDto(
    val id: Int,
    val empCode: String,
    val fullName: String,
    val email: String,
    val phone: String,
    val gender: String,
    val department: String,
    val designation: String,
    val reportingManager: String,
    val profilePhotoUrl: String,
    val dateOfJoining: String,
    val employmentType: String,
    val status: String
)

data class ProfileUpdateRequest(
    val phone: String?,
    val address: String?,
    val city: String?,
    val state: String?,
    val pinCode: String?,
    val emergencyContact: String?
)

// ── Attendance ──
data class PunchRequest(val latitude: Double, val longitude: Double, val address: String?)
data class PunchResponse(val message: String, val time: String, val lateByMinutes: Int? = null, val workingMinutes: Int? = null)
data class AttendanceDayDto(
    val date: String,
    val punchIn: String?,
    val punchOut: String?,
    val workingMinutes: Int,
    val lateByMinutes: Int,
    val status: String
)
data class AttendanceSummaryDto(
    val presentDays: Int,
    val absentDays: Int,
    val halfDays: Int,
    val leaveDays: Int,
    val holidays: Int,
    val weekOffs: Int,
    val lateDays: Int,
    val totalWorkingMinutes: Int,
    val avgWorkingHours: Double
)
data class RegularizeRequest(val date: String, val punchIn: String, val punchOut: String, val reason: String)

// ── Leave ──
data class LeaveTypeDto(
    val id: Int,
    val name: String,
    val code: String,
    val color: String,
    val defaultDays: Int,
    val isPaid: Boolean,
    val allowHalfDay: Boolean
)
data class LeaveBalanceDto(
    val leaveTypeId: Int,
    val leaveTypeName: String,
    val color: String,
    val entitled: Double,
    val used: Double,
    val balance: Double,
    val isPaid: Boolean
)
data class LeaveApplyRequest(
    val leaveTypeId: Int,
    val fromDate: String,
    val toDate: String,
    val isHalfDay: Boolean,
    val halfDayType: String?,
    val reason: String,
    val emergencyContact: String?
)
data class LeaveRequestDto(
    val id: Long,
    val leaveType: String,
    val leaveTypeColor: String,
    val fromDate: String,
    val toDate: String,
    val totalDays: Double,
    val isHalfDay: Boolean,
    val reason: String,
    val status: String,
    val approverRemarks: String?,
    val appliedAt: String,
    val actionDate: String?
)
data class LeaveActionRequest(val action: String, val remarks: String?)

// ── Payslip ──
data class PayslipListDto(
    val id: Long,
    val payPeriod: String,
    val month: Int,
    val year: Int,
    val netPay: Double,
    val status: String,
    val generatedAt: String
)
data class PayslipDetailDto(
    val id: Long,
    val payPeriod: String,
    val employeeName: String,
    val empCode: String,
    val department: String,
    val designation: String,
    val month: Int,
    val year: Int,
    val basicSalary: Double,
    val hra: Double,
    val conveyanceAllowance: Double,
    val medicalAllowance: Double,
    val specialAllowance: Double,
    val otherEarnings: Double,
    val grossEarnings: Double,
    val pf: Double,
    val esi: Double,
    val professionalTax: Double,
    val tds: Double,
    val loanDeduction: Double,
    val otherDeductions: Double,
    val totalDeductions: Double,
    val netPay: Double,
    val paidDays: Int,
    val lopDays: Int,
    val status: String
)

// ── Expense ──
data class ExpenseCategoryDto(val id: Int, val name: String, val maxLimit: Double?, val requiresReceipt: Boolean)
data class ExpenseSubmitRequest(
    val categoryId: Int,
    val title: String,
    val description: String?,
    val amount: Double,
    val expenseDate: String,
    val receiptUrl: String?
)
data class ExpenseDto(
    val id: Long,
    val category: String,
    val title: String,
    val amount: Double,
    val expenseDate: String,
    val status: String,
    val approvedAmount: Double?,
    val approverRemarks: String?,
    val submittedAt: String
)

// ── Directory ──
data class DirectoryEntryDto(
    val id: Int,
    val fullName: String,
    val empCode: String,
    val department: String,
    val designation: String,
    val email: String,
    val phone: String,
    val profilePhotoUrl: String
)

// ── Helpdesk ──
data class CreateTicketRequest(val category: String, val subject: String, val description: String, val priority: String)
data class TicketDto(
    val id: Long,
    val ticketNo: String,
    val category: String,
    val subject: String,
    val description: String,
    val priority: String,
    val status: String,
    val resolution: String?,
    val createdAt: String,
    val resolvedAt: String?,
    val comments: List<TicketCommentDto>?
)
data class TicketCommentDto(val id: Long, val userName: String, val comment: String, val createdAt: String)
data class AddCommentRequest(val comment: String)

// ── Notification ──
data class NotificationDto(val id: Long, val title: String, val body: String, val type: String, val isRead: Boolean, val createdAt: String)
data class NotificationPage(val total: Int, val page: Int, val pageSize: Int, val data: List<NotificationDto>)
data class UnreadCountResponse(val count: Int)

// ── Announcement & Holiday ──
data class AnnouncementDto(val id: Int, val title: String, val content: String, val category: String, val priority: String, val publishedAt: String)
data class HolidayDto(val id: Int, val name: String, val date: String, val type: String)

// ── Dashboard ──
data class EmployeeDashboardDto(
    val fullName: String,
    val department: String,
    val designation: String,
    val profilePhotoUrl: String,
    val todayAttendance: AttendanceDayDto?,
    val leaveBalance: List<LeaveBalanceDto>,
    val pendingLeaves: Int,
    val pendingExpenses: Int,
    val unreadNotifications: Int,
    val openTickets: Int,
    val recentAnnouncements: List<AnnouncementDto>,
    val upcomingHolidays: List<HolidayDto>
)

// ── Generic ──
data class MessageResponse(val message: String)
data class IdResponse(val message: String, val id: Long? = null)
