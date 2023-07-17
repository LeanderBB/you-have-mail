package dev.lbeernaert.youhavemail.screens

import android.content.Context
import android.util.Log
import android.widget.Toast
import androidx.compose.runtime.Composable
import androidx.compose.ui.res.stringResource
import androidx.navigation.NavType
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.navArgument
import dev.lbeernaert.youhavemail.AppState
import dev.lbeernaert.youhavemail.Backend
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.RequestErrorCategory
import dev.lbeernaert.youhavemail.ServiceException
import dev.lbeernaert.youhavemail.service.ServiceWrapper
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

const val navLogTag = "nav"

@Composable
fun MainNavController(
    context: Context,
    serviceWrapper: ServiceWrapper,
    appState: AppState,
    requestPermissions: () -> Unit
) {
    val navController = rememberNavController()

    val onLoginClicked: suspend (String, String, Backend, String?) -> Unit =
        { email, password, backend, hv_data ->
            try {
                val account = withContext(Dispatchers.Default) {
                    val account = serviceWrapper.newAccount(backend, email)
                    account.login(password, hv_data)
                    account
                }

                if (account.isAwaitingTotp()) {
                    navController.navigate(Routes.TOTP.route)
                } else {
                    serviceWrapper.addAccount(account)
                    navController.popBackStack(Routes.Main.route, false)
                }
            } catch (e: ServiceException) {
                when (e) {
                    is ServiceException.HvCaptchaRequest -> {
                        // Avoid Loop
                        if (hv_data != null) {
                            throw ServiceException.Unknown(msg = "Captcha Request Loop")
                        }
                        val route =
                            Routes.captchaLoginRouteForBackend(backend = backend)
                        if (route != null) {
                            appState.CaptchaHTML = e.msg
                            navController.navigate(Routes.ProtonCaptcha.route)
                        } else {
                            throw ServiceException.RequestException(
                                category = RequestErrorCategory.API,
                                msg = "No Captcha implementation for the current backend"
                            )
                        }

                    }

                    else -> {
                        throw e
                    }
                }
            }
        }
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
            val accountEmail = args?.getString("email")!!
            if (backendIndex == null) {
                Log.e(navLogTag, "No backend index selected, returning to main screen")
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
                        appState.clearLoginState()
                        appState.UserEmail = email
                        appState.UserBackend = backend
                        appState.UserPassword = password
                        onLoginClicked(email, password, backend, null)
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
            Main(serviceWrapper, navController, requestPermissions) {
                navController.navigate(Routes.Settings.route)
            }
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
                Log.e(navLogTag, "No account index selected, returning to main screen")
                navController.popBackStack(Routes.Main.route, false)
            }

            val account = serviceWrapper.getAccount(accountIndex!!)
            if (account == null) {
                Log.e(navLogTag, "Account not found, return to main screen")
                navController.popBackStack(Routes.Main.route, false)
            } else {
                val email = account.email
                AccountInfo(
                    account = account,
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
                            serviceWrapper.setInLoginProxy(account.proxy.value)
                            navController.navigate(Routes.newLoginRoute(backendIndex, email))
                        } else {
                            Log.e(navLogTag, "Could not find backend named: ${account.backend}")
                        }
                    },
                    onDelete = {
                        withContext(Dispatchers.Default) {
                            serviceWrapper.removeAccount(email)
                        }
                        navController.popBackStack(Routes.Main.route, false)
                    },
                    onProxyClicked = {
                        navController.navigate(Routes.newAccountProxyRoute(accountIndex))
                    }
                )
            }
        }

        composable(Routes.Settings.route) {
            Settings(serviceWrapper,
                onBackClicked = {
                    navController.popBackStack()
                },
                onPollIntervalUpdate = { interval ->
                    serviceWrapper.setPollInterval(interval)
                }
            )
        }

        composable(
            Routes.ProxyLogin.route,
            arguments = listOf(navArgument("index") { type = NavType.IntType })
        ) {
            val args = it.arguments
            val backendIndex = args?.getInt("index")
            if (backendIndex == null) {
                Log.e(navLogTag, "No backend index selected, returning to main screen")
                navController.popBackStack(Routes.Main.route, false)
            } else {
                ProxyScreen(
                    onBackClicked = {
                        navController.popBackStack()
                    },
                    applyButtonText = stringResource(id = R.string.next),
                    onApplyClicked = { proxy ->
                        withContext(Dispatchers.Default) {
                            serviceWrapper.checkProxy(backendIndex, proxy)
                        }
                        serviceWrapper.setInLoginProxy(proxy)
                        navController.navigate(Routes.newLoginRoute(backendIndex, null))
                    },
                    proxy = null,
                    isLoginRequest = true,
                )
            }
        }

        composable(
            Routes.ProxySettings.route,
            arguments = listOf(navArgument("index") { type = NavType.IntType })
        ) {
            val accountIndex = it.arguments?.getInt("index")
            if (accountIndex == null) {
                Log.e(navLogTag, "No account index selected, returning to main screen")
                navController.popBackStack(Routes.Main.route, false)
            }

            val account = serviceWrapper.getAccount(accountIndex!!)
            if (account == null) {
                Log.e(navLogTag, "Account not found, return to main screen")
                navController.popBackStack(Routes.Main.route, false)
            } else {
                ProxyScreen(
                    onBackClicked = {
                        navController.popBackStack()
                    },
                    applyButtonText = stringResource(id = R.string.apply),
                    onApplyClicked = { proxy ->
                        withContext(Dispatchers.Default) {
                            serviceWrapper.setAccountProxy(account.email, proxy = proxy)
                        }
                        navController.popBackStack()
                    },
                    proxy = account.proxy.value,
                    isLoginRequest = false,
                )
            }
        }
        composable(Routes.ProtonCaptcha.route) {
            ProtonCaptchaScreen(
                onBackClicked = {
                    navController.popBackStack()
                },
                onCaptchaSuccess = {
                    if (appState.UserBackend == null || appState.UserEmail == null || appState.UserEmail == null) {
                        throw Exception("Invalid state")
                    }
                    onLoginClicked(
                        appState.UserEmail!!,
                        appState.UserPassword!!,
                        appState.UserBackend!!,
                        it
                    )
                },
                onCaptchaFail = {
                    Toast.makeText(context, it, Toast.LENGTH_SHORT).show()
                    navController.popBackStack()
                },
                html = appState.CaptchaHTML!!
            )
        }
    }
}


