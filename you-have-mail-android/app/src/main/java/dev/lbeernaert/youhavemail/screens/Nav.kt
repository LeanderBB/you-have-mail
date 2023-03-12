package dev.lbeernaert.youhavemail.screens

import androidx.compose.runtime.Composable
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import dev.lbeernaert.youhavemail.Log
import dev.lbeernaert.youhavemail.service.ServiceWrapper
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

@Composable
fun MainNavController(serviceWrapper: ServiceWrapper, requestPermissions: () -> Unit) {
    val navController = rememberNavController()
    NavHost(navController = navController, startDestination = Routes.Main.route) {
        composable(
            Routes.Login.route,
            arguments = listOf(
                navArgument("backend") { type = NavType.IntType },
                navArgument("email") {
                    type = NavType.StringType
                    defaultValue = ""
                })
        ) {
            val args = it.arguments
            val backendIndex = args?.getInt("backend")
            var accountEmail = args?.getString("email")!!
            if (backendIndex == null) {
                Log.e("No backend index selected, returning to main screen")
                navController.popBackStack(Routes.Main.route, false)
            } else {
                val backend = serviceWrapper.getBackends()[backendIndex]
                Login(
                    accountEmail = accountEmail,
                    backendName = backend.name(),
                    onBackClicked = {
                        navController.popBackStack()
                    },
                    onLoginClicked = { email, password ->
                        val account = withContext(Dispatchers.Default) {
                            var account = serviceWrapper.newAccount(backend, email)
                            account.login(password)
                            account
                        }
                        if (account.isAwaitingTotp()) {
                            navController.navigate(Routes.TOTP.route)
                        } else {
                            serviceWrapper.addAccount(account)
                            navController.popBackStack(Routes.Main.route, false)
                        }
                    }
                )
            }
        }
        composable(Routes.TOTP.route) {
            val onTotpClicked: suspend (value: String) -> Unit =
                { totp ->
                    withContext(Dispatchers.Default) {
                        val account = serviceWrapper.getInLoginAccount()!!
                        account.submitTotp(totp)
                        serviceWrapper.addAccount(account)
                    }
                    navController.popBackStack(Routes.Main.route, false)
                }
            Totp(onBackClicked = {
                navController.popBackStack()
            }, onTotpClicked = onTotpClicked)
        }
        composable(Routes.Main.route) {
            Main(serviceWrapper, navController, requestPermissions)
        }
        composable(Routes.Backend.route) {
            BackendSelection(serviceWrapper = serviceWrapper, navController = navController)
        }

        composable(
            Routes.Account.route,
            arguments = listOf(navArgument("index") { type = NavType.IntType })
        ) {
            val accountIndex = it.arguments?.getInt("index")
            if (accountIndex == null) {
                Log.e("No account index selected, returning to main screen")
                navController.popBackStack(Routes.Main.route, false)
            }

            val account = serviceWrapper?.getAccount(accountIndex!!)
            if (account == null) {
                Log.e("Account not found, return to main screen")
                navController.popBackStack(Routes.Main.route, false)
            } else {
                val email = account.email
                AccountInfo(
                    accountEmail = email,
                    backendName = account.backend,
                    accountStatus = account.status,
                    onBackClicked = {
                        navController.popBackStack(Routes.Main.route, false)
                    },
                    onLogout = {
                        withContext(Dispatchers.Default) {
                            serviceWrapper.logoutAccount(email)
                        }
                    },
                    onLogin = {
                        val backendIndex = serviceWrapper.backendIndexByName(account.backend)
                        if (backendIndex != null) {
                            navController.navigate(Routes.newLoginRoute(backendIndex, email))
                        } else {
                            Log.e("Could not find backend named: ${account.backend}")
                        }
                    },
                    onDelete = {
                        withContext(Dispatchers.Default) {
                            serviceWrapper.removeAccount(email)
                        }
                        navController.popBackStack(Routes.Main.route, false)
                    }
                )
            }
        }
    }
}


