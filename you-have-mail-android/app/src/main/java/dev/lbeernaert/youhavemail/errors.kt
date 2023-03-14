package dev.lbeernaert.youhavemail

fun serviceExceptionToErrorStr(e: ServiceException, email: String?): String {
    when (e) {
        is ServiceException.RpcFailed -> {
            return "Failed to communicate with service"
        }
        is ServiceException.AccountAlreadyActive -> {
            return if (email != null) {
                "Account $email is already active"
            } else {
                "Account is already active"
            }
        }
        is ServiceException.InvalidAccountState -> {
            return if (email != null) {
                "$email is in an invalid state"
            } else {
                return "Invalid account state"
            }
        }
        is ServiceException.RequestException -> {
            return e.msg
        }
        is ServiceException.LoggedOut -> {
            return if (email != null) {
                "Account $email is logged out"
            } else {
                "Account is logged out"
            }
        }
        is ServiceException.Offline -> {
            return if (email != null) {
                "Account $email is offline"
            } else {
                "Account is offline"
            }
        }
        is ServiceException.Config -> {
            return "Configuration error"
        }
        is ServiceException.AccountNotFound -> {
            return "Account not found"
        }
        else -> {
            return "Unknown error occurred"
        }
    }
}