package dev.lbeernaert.youhavemail.screens

sealed class Routes(val route: String) {
    object Account : Routes("Account/{index}")
    object Login : Routes("Login/{backend}?email={email}")
    object TOTP : Routes("TOTP")
    object Main : Routes("Main")
    object Backend : Routes("Backend")

    object Settings : Routes("Settings")

    companion object {
        fun newLoginRoute(backendIndex: Int, email: String?): String {
            if (email != null) {
                return "Login/$backendIndex?email=$email"
            }

            return "Login/$backendIndex"
        }

        fun newAccountRoute(AccountIndex: Int): String {
            return "Account/$AccountIndex"
        }
    }
}