package dev.lbeernaert.youhavemail.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.material.Button
import androidx.compose.material.ButtonDefaults
import androidx.compose.material.FabPosition
import androidx.compose.material.FloatingActionButton
import androidx.compose.material.Icon
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Scaffold
import androidx.compose.material.Text
import androidx.compose.material.TopAppBar
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material.icons.filled.Settings
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.navigation.NavController
import dev.lbeernaert.youhavemail.Account
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.app.State
import dev.lbeernaert.youhavemail.app.accountStatusString
import java.util.Timer
import java.util.TimerTask

@Composable
fun Main(
    state: State,
    navController: NavController,
    requestPermissions: () -> Unit,
    onSettingsClicked: () -> Unit,
    onPollClicked: () -> Unit,
) {
    AccountList(
        state = state,
        navController = navController,
        requestPermissions = requestPermissions,
        onSettingsClicked = onSettingsClicked,
        onPollClicked = onPollClicked,
    )
}


@Composable
fun AccountList(
    state: State,
    navController: NavController,
    requestPermissions: () -> Unit,
    onSettingsClicked: () -> Unit,
    onPollClicked: () -> Unit,
) {
    val accounts by state.accounts().collectAsState()
    var pollActive by remember { mutableStateOf(true) }

    Scaffold(topBar = {
        TopAppBar(title = { Text(text = stringResource(id = R.string.app_name)) }, actions = {
            Button(
                onClick = {
                    onPollClicked()
                    pollActive = false
                    Timer().schedule(
                        object : TimerTask() {
                            override fun run() {
                                pollActive = true
                            }
                        }, 15000
                    )
                },
                colors = ButtonDefaults.outlinedButtonColors(),
                enabled = pollActive,
            ) {
                Icon(
                    Icons.Filled.Refresh,
                    contentDescription = "Poll accounts",
                    modifier = Modifier.size(30.dp)
                )
            }
            Spacer(modifier = Modifier.size(10.dp))
            Button(
                onClick = onSettingsClicked,
                colors = ButtonDefaults.outlinedButtonColors()
            ) {
                Icon(
                    Icons.Filled.Settings,
                    contentDescription = "Settings",
                    modifier = Modifier.size(30.dp)
                )
            }
        })
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
                        itemsIndexed(accounts) { _, account ->
                            ActiveAccount(account = account, onClicked = {
                                navController.navigate(Routes.newAccountRoute(it))
                            })
                        }
                    }
                }
            }
        })
}


@Composable
fun ActiveAccount(account: Account, onClicked: (String) -> Unit) {
    Row(
        modifier = Modifier
            .padding(10.dp)
            .fillMaxWidth()
            .clickable { onClicked(account.email()) },
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

            Text(
                text = stringResource(id = R.string.status, accountStatusString(account)),
                style = MaterialTheme.typography.body2
            )
        }
    }
}


