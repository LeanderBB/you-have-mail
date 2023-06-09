package dev.lbeernaert.youhavemail.screens

sealed class Routes(val route: String) {
    object Account : Routes("Account/{index}")
    object Login : Routes("Login/{backend}?email={email}")
    object TOTP : Routes("TOTP")
    object Main : Routes("Main")
    object Backend : Routes("Backend")

    object Settings : Routes("Settings")
    object ProxyLogin : Routes("ProxyLogin/{index}")
    object ProxySettings : Routes("ProxySettings/{index}")
    object ProtonCaptcha : Routes("ProtonCaptcha")

    companion object {
        fun newLoginRoute(backendIndex: Int, email: String?): String {
            if (email != null) {
                return "Login/$backendIndex?email=$email"
            }

            return "Login/$backendIndex"
        }

        fun newProxyLoginRoute(backendIndex: Int): String {
            return "ProxyLogin/$backendIndex"
        }

        fun newAccountRoute(accountIndex: Int): String {
            return "Account/$accountIndex"
        }

        fun newAccountProxyRoute(accountIndex: Int): String {
            return "ProxySettings/$accountIndex"
        }

        fun captchaLoginRouteForBackend(backend: dev.lbeernaert.youhavemail.Backend): Routes? {
            return when (backend.name()) {
                "Proton Mail" -> ProtonCaptcha
                "Proton Mail V-Other" -> ProtonCaptcha
                else -> {
                    null
                }
            }
        }
    }
}