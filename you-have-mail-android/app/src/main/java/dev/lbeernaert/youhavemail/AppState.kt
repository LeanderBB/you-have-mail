package dev.lbeernaert.youhavemail

class AppState {
    var UserBackend: Backend? = null
    var UserEmail: String? = null
    var UserPassword: String? = null
    var CaptchaHTML: String? = null

    fun clearLoginState() {
        UserBackend = null
        UserEmail = null
        UserPassword = null
        CaptchaHTML = null
    }

    fun onDestroy() {
    }
}

