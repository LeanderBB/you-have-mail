package dev.lbeernaert.youhavemail.screens

import androidx.compose.foundation.layout.*
import androidx.compose.material.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.runtime.Composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.compose.composable
import androidx.navigation.compose.NavHost
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.res.stringResource
import androidx.navigation.NavController
import androidx.navigation.NavType
import androidx.navigation.navArgument
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.ServiceView


@Composable
fun Main(service: ServiceView?, navController: NavController) {
    if (service == null) {
        WaitingOnService()
    } else {
        AccountList(service = service, navController = navController)
    }
}


@Composable
fun WaitingOnService() {
    Column(
        modifier = Modifier
            .padding(10.dp)
            .fillMaxSize(),
        verticalArrangement = Arrangement.Center,
        horizontalAlignment = Alignment.CenterHorizontally,

        ) {
        CircularProgressIndicator()
        Spacer(modifier = Modifier.padding(10.dp))
        Text(text = stringResource(id = R.string.connecting_to_service))
    }
}

@Composable
fun AccountList(service: ServiceView, navController: NavController) {
    val accounts = service.getAccounts()

    Scaffold(topBar = {
        TopAppBar(title = { Text(text = stringResource(id = R.string.app_name)) })
    },
        floatingActionButtonPosition = FabPosition.End,
        floatingActionButton = {
            FloatingActionButton(onClick = {
                navController.navigate(Routes.Backend.route)
            }) {
                Icon(Icons.Filled.Add, "")
            }
        },
        content = { padding ->
            Column(
                modifier = Modifier
                    .padding(padding)
                    .fillMaxSize(),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally,

                ) {
                if (accounts.isEmpty()) {
                    Text(text = stringResource(id = R.string.no_accounts))
                }
            }
        })
}


@Composable
fun MainNavController(service: ServiceView) {
    val navController = rememberNavController()

    NavHost(navController = navController, startDestination = Routes.Main.route) {
        composable(
            Routes.Login.route,
            arguments = listOf(navArgument("backend") { type = NavType.IntType })
        ) {
            val backendIndex = it.arguments?.getInt("backend")
            if (backendIndex == null) {
                //TODO:
            } else {
                Login(
                    serviceView = service,
                    navController = navController,
                    backendIndex = backendIndex
                )
            }
        }
        composable(Routes.TOTP.route) {

        }
        composable(Routes.Main.route) {
            Main(service, navController)
        }
        composable(Routes.Backend.route) {
            BackendSelection(serviceView = service, navController = navController)
        }
    }
}
