package dev.lbeernaert.youhavemail.screens

import android.app.Activity
import android.util.Log
import android.widget.Toast
import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import androidx.navigation.NavController
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.app.State
import dev.lbeernaert.youhavemail.testProxy
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

const val navLogTag = "nav"

@Composable
fun MainNavController(
    context: Activity,
    state: State,
    requestPermissions: () -> Unit,
    onPollClicked: () -> Unit,
    onExportLogsClicked: () -> Unit
) {
    val navController = rememberNavController()

    NavHost(navController = navController, startDestination = Routes.Main.route) {
        // ------------------ LOGIN --------------------------------------------------------------
        composable(
            Routes.Login.route,
            arguments = listOf(
                navArgument("email") {
                    type = NavType.StringType
                    defaultValue = ""
                })
        ) {
            val args = it.arguments
            val accountEmail = args?.getString("email").orEmpty()
            if (state.mLoginSequence == null) {
                Log.e(navLogTag, "No backend index selected, returning to main screen")
                navController.popBackStack(Routes.Main.route, false)
            } else {
                Login(
                    accountEmail = accountEmail,
                    backendName = state.mLoginSequence!!.backendName(),
                    onBackClicked = {
                        popBack(navController)
                    },
                    onLoginClicked = { email, password ->
                        if(email.isBlank() or password.isEmpty()) {
                            Toast.makeText(
                                context,
                                R.string.empty_email_or_password_info,
                                Toast.LENGTH_SHORT
                            ).show()
                        } else if(!android.util.Patterns.EMAIL_ADDRESS.matcher(email).matches()) {
                            Toast.makeText(
                                context,
                                R.string.invalid_email_address,
                                Toast.LENGTH_SHORT
                            ).show()
                        }
                        else {
                            withContext(Dispatchers.Default) {
                                state.mLoginSequence!!.login(email, password)
                            }
                            state.mLoginSequence!!.next(navController)
                        }
                    }
                )
            }
        }
        // ------------------ TOTP ---------------------------------------------------------------
        composable(Routes.TOTP.route) {
            val onTotpClicked: suspend (value: String) -> Unit =
                { totp ->
                    withContext(Dispatchers.Default) {
                        state.mLoginSequence!!.totp(totp)
                    }
                    state.mLoginSequence!!.next(navController)
                }
            Totp(onBackClicked = {
                popBack(navController)
            }, onTotpClicked = onTotpClicked)
        }
        // ------------------ MAIN ---------------------------------------------------------------
        composable(Routes.Main.route) {
            Main(state, navController, requestPermissions, onSettingsClicked = {
                navController.navigate(Routes.Settings.route)
            }, onPollClicked)
        }
        // ------------------ BACKEND SELECTION --------------------------------------------------
        composable(Routes.Backend.route) {
            BackendSelection(state = state, navController = navController)
        }
        // ------------------ ACCOUNT DETAILS ----------------------------------------------------
        composable(
            Routes.Account.route,
            arguments = listOf(navArgument("email") { type = NavType.StringType })
        ) {
            val accountEmail = it.arguments?.getString("email")
            if (accountEmail == null) {
                Log.e(navLogTag, "No account index selected, returning to main screen")
                navController.popBackStack(Routes.Main.route, false)
            }
            val account = state.account(accountEmail!!)
            if (account == null) {
                Log.e(navLogTag, "Account not found, return to main screen")
                navController.popBackStack(Routes.Main.route, false)
            } else {
                AccountInfo(
                    account = account,
                    onBackClicked = {
                        navController.popBackStack(Routes.Main.route, false)
                    },
                    onLogout = {
                        withContext(Dispatchers.Default) {
                            state.logout(accountEmail)
                        }
                    },
                    onLogin = {
                        state.mLoginSequence =
                            state.newLoginSequence(account.backend(), account.proxy())
                        navController.navigate(Routes.newLoginRoute(accountEmail))
                    },
                    onDelete = {
                        withContext(Dispatchers.Default) {
                            state.delete(accountEmail)
                        }
                        navController.popBackStack(Routes.Main.route, false)
                    },
                    onProxyClicked = {
                        navController.navigate(Routes.newAccountProxyRoute(accountEmail))
                    }
                )
            }
        }
        // ------------------ SETTINGS -----------------------------------------------------------
        composable(Routes.Settings.route) {
            Settings(
                state = state,
                onBackClicked = {
                    navController.popBackStack()
                },
                onPollIntervalUpdate = { interval ->
                    state.setPollInterval(context, interval)
                },
                onExportLogsClicked = onExportLogsClicked
            )
        }
        // ------------------ PROXY LOGIN --------------------------------------------------------
        composable(
            Routes.ProxyLogin.route,
            arguments = listOf(navArgument("name") { type = NavType.StringType })
        ) {
            val args = it.arguments
            val backendName = args?.getString("name")
            if (backendName == null) {
                Log.e(navLogTag, "No backend name selected, returning to main screen")
                navController.popBackStack(Routes.Main.route, false)
            } else {
                ProxyScreen(
                    onBackClicked = {
                        popBack(navController)
                    },
                    applyButtonText = stringResource(id = R.string.next),
                    onApplyClicked = { proxy ->
                        withContext(Dispatchers.Default) {
                            if (proxy != null) {
                                testProxy(proxy)
                            }
                        }
                        state.mLoginSequence = state.newLoginSequence(backendName, proxy)
                        navController.navigate(Routes.newLoginRoute(null))
                    },
                    proxy = null,
                    isLoginRequest = true,
                )
            }
        }
        // ------------------ PROXY SETTINGS -----------------------------------------------------
        composable(
            Routes.ProxySettings.route,
            arguments = listOf(navArgument("email") { type = NavType.StringType })
        ) {
            val accountEmail = it.arguments?.getString("email")
            if (accountEmail == null) {
                Log.e(navLogTag, "No account index selected, returning to main screen")
                navController.popBackStack(Routes.Main.route, false)
            }

            val account = state.account(accountEmail!!)
            if (account == null) {
                Log.e(navLogTag, "Account not found, return to main screen")
                navController.popBackStack(Routes.Main.route, false)
            } else {
                ProxyScreen(
                    onBackClicked = {
                        popBack(navController)
                    },
                    applyButtonText = stringResource(id = R.string.apply),
                    onApplyClicked = { proxy ->
                        withContext(Dispatchers.Default) {
                            account.setProxy(proxy)
                        }
                        popBack(navController)
                    },
                    proxy = account.proxy(),
                    isLoginRequest = false,
                )
            }
        }
        // ------------------ PROTON CAPTCHA -----------------------------------------------------
        composable(Routes.ProtonCaptcha.route) {
            ProtonCaptchaScreen(
                onBackClicked = {
                    navController.popBackStack()
                },
                onCaptchaSuccess = {
                    withContext(Dispatchers.Default) {
                        state.mLoginSequence!!.protonCaptcha(it)
                    }
                    state.mLoginSequence!!.next(navController)
                },
                onCaptchaFail = {
                    Toast.makeText(context, it, Toast.LENGTH_SHORT).show()
                    popBack(navController)
                },
                html = state.mLoginSequence!!.protonCaptchaHtml(),
            )
        }
    }
}

fun popBack(navController: NavController) {
    // Always make sure the main route is present if everything is popped of the stack
    if (!navController.popBackStack()) {
        navController.navigate(Routes.Main.route)
    }
}
