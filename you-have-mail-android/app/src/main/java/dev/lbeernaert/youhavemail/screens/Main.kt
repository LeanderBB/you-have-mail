package dev.lbeernaert.youhavemail.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.navigation.NavController
import dev.lbeernaert.youhavemail.ObserverAccount
import dev.lbeernaert.youhavemail.ObserverAccountState
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.ServiceView
import dev.lbeernaert.youhavemail.components.BackgroundTask


@Composable
fun Main(service: ServiceView?, navController: NavController, requestPermissions: () -> Unit) {
    if (service == null) {
        BackgroundTask(text = stringResource(id = R.string.connecting_to_service))
    } else {
        AccountList(
            service = service,
            navController = navController,
            requestPermissions = requestPermissions
        )
    }
}


@Composable
fun AccountList(
    service: ServiceView,
    navController: NavController,
    requestPermissions: () -> Unit
) {
    val accounts = service.getAccounts()

    Scaffold(topBar = {
        TopAppBar(title = { Text(text = stringResource(id = R.string.app_name)) })
    },
        floatingActionButtonPosition = FabPosition.End,
        floatingActionButton = {
            FloatingActionButton(onClick = {
                requestPermissions()
                navController.navigate(Routes.Backend.route)
            }) {
                Icon(Icons.Filled.Add, "")
            }
        },
        content = { padding ->
            val isEmpty = accounts.isEmpty()
            Column(
                modifier = Modifier
                    .padding(padding)
                    .fillMaxSize(),
                verticalArrangement = if (isEmpty) {
                    Arrangement.Center
                } else {
                    Arrangement.Top
                },
                horizontalAlignment = if (isEmpty) {
                    Alignment.CenterHorizontally
                } else {
                    Alignment.Start
                }

            ) {
                if (accounts.isEmpty()) {
                    Text(text = stringResource(id = R.string.no_accounts))
                } else {
                    LazyColumn(
                        contentPadding = PaddingValues(
                            horizontal = 10.dp,
                            vertical = 10.dp
                        )
                    ) {
                        itemsIndexed(accounts) { index, account ->
                            ActiveAccount(account = account, index = index, onClicked = {
                                navController.navigate(Routes.newAccountRoute(it))
                            })
                        }
                    }
                }
            }
        })
}


@Composable
fun ActiveAccount(account: ObserverAccount, index: Int, onClicked: (Int) -> Unit) {
    Row(
        modifier = Modifier
            .padding(10.dp)
            .fillMaxWidth()
            .clickable { onClicked(index) },
    ) {
        val email = account.email()
        Column(
            verticalArrangement = Arrangement.Center,
            modifier = Modifier
                .size(60.dp)
                .background(MaterialTheme.colors.primary, MaterialTheme.shapes.large),
        ) {
            Text(
                modifier = Modifier.fillMaxWidth(),
                text = email.first().toString().uppercase(),
                textAlign = TextAlign.Center,
                style = MaterialTheme.typography.button,
                fontWeight = FontWeight.Bold,
                fontSize = 30.sp
            )
        }
        Spacer(modifier = Modifier.width(10.dp))
        Column(modifier = Modifier.fillMaxWidth()) {
            Text(
                text = email,
                style = MaterialTheme.typography.subtitle1,
                fontWeight = FontWeight.Bold
            )
            val statusString = when (account.state()) {
                ObserverAccountState.OFFLINE -> stringResource(id = R.string.status_offline)
                ObserverAccountState.LOGGED_OUT -> stringResource(id = R.string.status_logged_out)
                ObserverAccountState.ONLINE -> stringResource(id = R.string.status_online)
            }
            Text(
                text = stringResource(id = R.string.status, statusString),
                style = MaterialTheme.typography.body2
            )
        }
    }
}


