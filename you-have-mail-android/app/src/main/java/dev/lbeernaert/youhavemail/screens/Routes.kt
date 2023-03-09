package dev.lbeernaert.youhavemail.screens

sealed class Routes(val route: String) {
    object Account : Routes("Account/{index}")
    object Login : Routes("Login/{backend}")
    object TOTP : Routes("TOTP")
    object Main : Routes("Main")
    object Backend : Routes("Backend")

    companion object {
        fun newLoginRoute(backendIndex: Int): String {
            return "Login/$backendIndex"
        }

        fun newAccountRoute(AccountIndex: Int): String {
            return "Account/$AccountIndex"
        }
    }
}