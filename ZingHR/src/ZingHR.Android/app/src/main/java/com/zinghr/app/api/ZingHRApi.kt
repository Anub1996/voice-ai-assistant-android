package com.zinghr.app.api

import retrofit2.Response
import retrofit2.http.*

interface ZingHRApi {

    // ── Auth ──
    @POST("api/auth/login")
    suspend fun login(@Body req: LoginRequest): Response<LoginResponse>

    @POST("api/auth/pin-login")
    suspend fun pinLogin(@Body req: PinLoginRequest): Response<LoginResponse>

    @POST("api/auth/set-pin")
    suspend fun setPin(@Body req: SetPinRequest): Response<MessageResponse>

    @GET("api/auth/me")
    suspend fun getMe(): Response<ProfileDto>

    @POST("api/auth/change-password")
    suspend fun changePassword(@Body req: ChangePasswordRequest): Response<MessageResponse>

    @PUT("api/auth/device-token")
    suspend fun updateDeviceToken(@Body req: DeviceTokenRequest): Response<Unit>

    // ── Employee Profile ──
    @GET("api/employee/profile")
    suspend fun getProfile(): Response<ProfileDto>

    @PUT("api/employee/profile")
    suspend fun updateProfile(@Body req: ProfileUpdateRequest): Response<MessageResponse>

    // ── Dashboard ──
    @GET("api/dashboard/employee")
    suspend fun getEmployeeDashboard(): Response<EmployeeDashboardDto>

    // ── Attendance ──
    @POST("api/attendance/punch-in")
    suspend fun punchIn(@Body req: PunchRequest): Response<PunchResponse>

    @POST("api/attendance/punch-out")
    suspend fun punchOut(@Body req: PunchRequest): Response<PunchResponse>

    @GET("api/attendance/today")
    suspend fun getAttendanceToday(): Response<AttendanceDayDto>

    @GET("api/attendance/history")
    suspend fun getAttendanceHistory(
        @Query("month") month: Int,
        @Query("year") year: Int
    ): Response<List<AttendanceDayDto>>

    @GET("api/attendance/summary")
    suspend fun getAttendanceSummary(
        @Query("month") month: Int,
        @Query("year") year: Int
    ): Response<AttendanceSummaryDto>

    @POST("api/attendance/regularize")
    suspend fun regularizeAttendance(@Body req: RegularizeRequest): Response<MessageResponse>

    // ── Leave ──
    @GET("api/leave/types")
    suspend fun getLeaveTypes(): Response<List<LeaveTypeDto>>

    @GET("api/leave/balance")
    suspend fun getLeaveBalance(@Query("year") year: Int? = null): Response<List<LeaveBalanceDto>>

    @POST("api/leave/apply")
    suspend fun applyLeave(@Body req: LeaveApplyRequest): Response<IdResponse>

    @GET("api/leave/history")
    suspend fun getLeaveHistory(@Query("year") year: Int? = null): Response<List<LeaveRequestDto>>

    @DELETE("api/leave/{id}")
    suspend fun cancelLeave(@Path("id") id: Long): Response<MessageResponse>

    @GET("api/leave/pending")
    suspend fun getPendingLeaves(): Response<List<LeaveRequestDto>>

    @PUT("api/leave/{id}/action")
    suspend fun leaveAction(@Path("id") id: Long, @Body req: LeaveActionRequest): Response<MessageResponse>

    // ── Payslip ──
    @GET("api/payslip")
    suspend fun getPayslips(@Query("year") year: Int? = null): Response<List<PayslipListDto>>

    @GET("api/payslip/{id}")
    suspend fun getPayslipDetail(@Path("id") id: Long): Response<PayslipDetailDto>

    // ── Expense ──
    @GET("api/expense/categories")
    suspend fun getExpenseCategories(): Response<List<ExpenseCategoryDto>>

    @POST("api/expense")
    suspend fun submitExpense(@Body req: ExpenseSubmitRequest): Response<IdResponse>

    @GET("api/expense")
    suspend fun getExpenses(
        @Query("status") status: String? = null,
        @Query("year") year: Int? = null
    ): Response<List<ExpenseDto>>

    // ── Directory ──
    @GET("api/directory")
    suspend fun getDirectory(
        @Query("search") search: String? = null,
        @Query("departmentId") departmentId: Int? = null
    ): Response<List<DirectoryEntryDto>>

    @GET("api/directory/{id}")
    suspend fun getEmployeeProfile(@Path("id") id: Int): Response<ProfileDto>

    @GET("api/directory/departments")
    suspend fun getDepartments(): Response<List<DepartmentDto>>

    // ── Helpdesk ──
    @POST("api/helpdesk")
    suspend fun createTicket(@Body req: CreateTicketRequest): Response<IdResponse>

    @GET("api/helpdesk")
    suspend fun getMyTickets(@Query("status") status: String? = null): Response<List<TicketDto>>

    @GET("api/helpdesk/{id}")
    suspend fun getTicketDetail(@Path("id") id: Long): Response<TicketDto>

    @POST("api/helpdesk/{id}/comment")
    suspend fun addTicketComment(@Path("id") id: Long, @Body req: AddCommentRequest): Response<MessageResponse>

    // ── Notifications ──
    @GET("api/notification")
    suspend fun getNotifications(
        @Query("unreadOnly") unreadOnly: Boolean? = null,
        @Query("page") page: Int = 1
    ): Response<NotificationPage>

    @GET("api/notification/unread-count")
    suspend fun getUnreadCount(): Response<UnreadCountResponse>

    @PUT("api/notification/{id}/read")
    suspend fun markNotificationRead(@Path("id") id: Long): Response<Unit>

    @PUT("api/notification/read-all")
    suspend fun markAllNotificationsRead(): Response<MessageResponse>

    // ── Announcements & Holidays ──
    @GET("api/announcement")
    suspend fun getAnnouncements(): Response<List<AnnouncementDto>>

    @GET("api/holiday")
    suspend fun getHolidays(@Query("year") year: Int? = null): Response<List<HolidayDto>>

    @GET("api/holiday/upcoming")
    suspend fun getUpcomingHolidays(): Response<List<HolidayDto>>
}

data class DepartmentDto(val id: Int, val name: String, val code: String, val employeeCount: Int)
