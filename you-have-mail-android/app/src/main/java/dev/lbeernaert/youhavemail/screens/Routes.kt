package dev.lbeernaert.youhavemail.screens

sealed class Routes(val route: String) {
    object Account : Routes("Account/{email}")
    object Login : Routes("Login?email={email}")
    object TOTP : Routes("TOTP")
    object Main : Routes("Main")
    object Backend : Routes("Backend")

    object Settings : Routes("Settings")
    object ProxyLogin : Routes("ProxyLogin/{name}")
    object ProxySettings : Routes("ProxySettings/{email}")
    object ProtonCaptcha : Routes("ProtonCaptcha")

    companion object {
        fun newLoginRoute(email: String?): String {
            if (email != null) {
                return "Login?email=$email"
            }

            return "Login"
        }

        fun newProxyLoginRoute(backendEmail: String): String {
            return "ProxyLogin/$backendEmail"
        }

        fun newAccountRoute(accountEmail: String): String {
            return "Account/$accountEmail"
        }

        fun newAccountProxyRoute(accountEmail: String): String {
            return "ProxySettings/$accountEmail"
        }
    }
}