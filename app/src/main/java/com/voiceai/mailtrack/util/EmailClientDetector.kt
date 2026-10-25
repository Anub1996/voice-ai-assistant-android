package com.voiceai.mailtrack.util

object EmailClientDetector {

    private val EMAIL_PACKAGES = mapOf(
        "com.google.android.gm" to "Gmail",
        "com.microsoft.office.outlook" to "Outlook",
        "com.yahoo.mobile.client.android.mail" to "Yahoo Mail",
        "com.samsung.android.email.provider" to "Samsung Email",
        "com.aol.mobile.aolapp" to "AOL",
        "org.mozilla.thunderbird" to "Thunderbird",
        "me.bluemail.mail" to "BlueMail",
        "com.easilydo.mail" to "Edison Mail",
        "com.protonmail.android" to "ProtonMail",
        "ch.protonmail.android" to "ProtonMail",
        "com.tutanota.tutanota" to "Tutanota"
    )

    fun isEmailPackage(packageName: String): Boolean {
        return EMAIL_PACKAGES.containsKey(packageName)
    }

    fun getClientName(packageName: String): String {
        return EMAIL_PACKAGES[packageName] ?: "Unknown"
    }

    fun getAllPackages(): Set<String> = EMAIL_PACKAGES.keys
}
