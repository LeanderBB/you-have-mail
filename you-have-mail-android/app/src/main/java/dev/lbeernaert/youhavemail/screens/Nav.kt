package dev.lbeernaert.youhavemail.screens

import androidx.compose.runtime.Composable
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import dev.lbeernaert.youhavemail.Log
import dev.lbeernaert.youhavemail.ServiceView
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

@Composable
fun MainNavController(serviceView: ServiceView) {
    val navController = rememberNavController()
    NavHost(navController = navController, startDestination = Routes.Main.route) {
        composable(
            Routes.Login.route,
            arguments = listOf(navArgument("backend") { type = NavType.IntType })
        ) {
            val backendIndex = it.arguments?.getInt("backend")
            if (backendIndex == null) {
                Log.e("No backend index selected, returning to main screen")
                navController.popBackStack(Routes.Main.route, false)
            } else {
                val backend = serviceView.getBackends()[backendIndex]
                Login(
                    backendName = backend.name(),
                    onBackClicked = {
                        navController.popBackStack()
                    },
                    onLoginClicked = { email, password ->
                        val account = withContext(Dispatchers.Default) {
                            val service = serviceView.getService()!!
                            var account = service.newAccount(backend, email)
                            serviceView.setInLoginAccount(account)
                            account.login(password)
                            account
                        }
                        if (account.isAwaitingTotp()) {
                            navController.navigate(Routes.TOTP.route)
                        } else {
                            serviceView.getService()!!.addAccount(account)
                            serviceView.requiresAccountRefresh()
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
                        val account = serviceView.getInLoginAccount()!!
                        account.submitTotp(totp)
                        serviceView.getService()!!.addAccount(account)
                        serviceView.requiresAccountRefresh()
                    }
                    navController.popBackStack(Routes.Main.route, false)
                }
            Totp(onBackClicked = {
                navController.popBackStack()
            }, onTotpClicked = onTotpClicked)
        }
        composable(Routes.Main.route) {
            Main(serviceView, navController)
        }
        composable(Routes.Backend.route) {
            BackendSelection(serviceView = serviceView, navController = navController)
        }

        composable(
            Routes.Account.route,
            arguments = listOf(navArgument("index") { type = NavType.IntType })
        ) {
            val accounts = serviceView.getAccounts()
            val accountIndex = it.arguments?.getInt("index")
            if (accountIndex == null || accountIndex >= accounts.size) {
                Log.e("No account index selected, returning to main screen")
                navController.popBackStack(Routes.Main.route, false)
            } else {
                val account = accounts[accountIndex]
                val email = account.email()
                val service = serviceView.getService()!!
                AccountInfo(
                    accountEmail = email,
                    backendName = account.backend(),
                    accountState = account.state(),
                    onBackClicked = {
                        navController.popBackStack(Routes.Main.route, false)
                    },
                    onLogout = {
                        withContext(Dispatchers.Default) {
                            service.logoutAccount(email)
                            serviceView.requiresAccountRefresh()
                        }
                    },
                    onLogin = {
                        val backends = service.getBackends()
                        for (b in backends.listIterator().withIndex()) {
                            if (b.value.name() == account.backend()) {
                                navController.navigate(Routes.newLoginRoute(b.index))
                            }
                        }
                        Log.e("Could not find backend named: ${account.backend()}")
                    },
                    onDelete = {
                        withContext(Dispatchers.Default) {
                            service.removeAccount(email)
                            serviceView.requiresAccountRefresh()
                        }
                        navController.popBackStack(Routes.Main.route, false)
                    }
                )
            }
        }
    }
}


