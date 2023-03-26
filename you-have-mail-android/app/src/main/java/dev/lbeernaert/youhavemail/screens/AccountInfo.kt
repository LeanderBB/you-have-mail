package dev.lbeernaert.youhavemail.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.Divider
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import dev.lbeernaert.youhavemail.ObserverAccountStatus
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.components.ActionButton
import dev.lbeernaert.youhavemail.components.AsyncScreen
import dev.lbeernaert.youhavemail.service.AccountActivity


@Composable
fun AccountInfo(
    accountEmail: String,
    backendName: String,
    accountStatus: ObserverAccountStatus,
    activity: List<AccountActivity>,
    onBackClicked: () -> Unit,
    onLogout: suspend () -> Unit,
    onLogin: () -> Unit,
    onDelete: suspend () -> Unit,
    onProxyClicked: () -> Unit,
) {
    val accountState = remember { mutableStateOf(accountStatus) }

    AsyncScreen(
        title = stringResource(id = R.string.account_title),
        onBackClicked = onBackClicked
    ) { padding, runTask ->

        val logOutBackgroundLabel = stringResource(id = R.string.logging_out)
        val onLogoutImpl: () -> Unit = {
            runTask(logOutBackgroundLabel) {
                onLogout()
                accountState.value = ObserverAccountStatus.LOGGED_OUT
            }
        }

        val onDeleteImpl: () -> Unit = {
            runTask(logOutBackgroundLabel) {
                onDelete()
            }
        }

        Column(
            modifier = Modifier
                .padding(padding)
                .padding(20.dp)
                .fillMaxSize()
                .verticalScroll(rememberScrollState(), true),
            verticalArrangement = Arrangement.Top,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {

            Text(
                text = accountEmail,
                modifier = Modifier.fillMaxWidth(),
                style = MaterialTheme.typography.h4,
            )

            Spacer(modifier = Modifier.height(20.dp))

            Text(
                text = "Backend: $backendName",
                modifier = Modifier.fillMaxWidth()
            )

            Spacer(modifier = Modifier.height(20.dp))

            val statusString = when (accountState.value) {
                ObserverAccountStatus.OFFLINE -> stringResource(id = R.string.status_offline)
                ObserverAccountStatus.LOGGED_OUT -> stringResource(id = R.string.status_logged_out)
                ObserverAccountStatus.ONLINE -> stringResource(id = R.string.status_online)
                ObserverAccountStatus.ERROR -> stringResource(id = R.string.status_online)
            }
            Text(
                text = stringResource(id = R.string.status, statusString),
                modifier = Modifier.fillMaxWidth()
            )

            Spacer(modifier = Modifier.height(40.dp))

            ActionButton(text = stringResource(id = R.string.proxy_settings), onProxyClicked)

            Spacer(modifier = Modifier.height(40.dp))

            if (accountState.value == ObserverAccountStatus.LOGGED_OUT) {
                ActionButton(
                    text = stringResource(id = R.string.login),
                    onClick = onLogin
                )
            } else {
                ActionButton(
                    text = stringResource(id = R.string.logout),
                    onClick = onLogoutImpl,
                )
            }

            Spacer(modifier = Modifier.height(20.dp))

            ActionButton(text = stringResource(id = R.string.delete_account), onDeleteImpl)

            Spacer(modifier = Modifier.height(40.dp))

            Divider()

            Spacer(modifier = Modifier.height(20.dp))

            Text(
                text = stringResource(id = R.string.account_activity),
                modifier = Modifier.fillMaxWidth()
            )

            Spacer(modifier = Modifier.height(20.dp))

            for (a in activity.reversed()) {
                Text(
                    text = a.dateTime.toString(),
                    modifier = Modifier.fillMaxWidth(),
                    style = MaterialTheme.typography.subtitle2
                )
                Text(
                    text = a.status,
                    modifier = Modifier.fillMaxWidth(),
                    style = MaterialTheme.typography.caption,
                )
                Spacer(modifier = Modifier.height(20.dp))
            }
        }

    }
}