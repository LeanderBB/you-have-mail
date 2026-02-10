package dev.lbeernaert.youhavemail.screens

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.lbeernaert.youhavemail.Account
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.app.accountStatusString
import dev.lbeernaert.youhavemail.components.ActionButton
import dev.lbeernaert.youhavemail.components.AsyncScreen


@Composable
fun AccountInfo(
    account: Account,
    onBackClicked: () -> Unit,
    onLogout: suspend () -> Unit,
    onLogin: () -> Unit,
    onDelete: suspend () -> Unit,
    onProxyClicked: () -> Unit,
) {

    AsyncScreen(
        title = stringResource(id = R.string.account_title),
        onBackClicked = onBackClicked
    ) { padding, runTask ->

        val logOutBackgroundLabel = stringResource(id = R.string.logging_out)
        val onLogoutImpl: () -> Unit = {
            runTask(logOutBackgroundLabel) {
                onLogout()
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
                text = "Email",
                modifier = Modifier.fillMaxWidth(),
                fontWeight = FontWeight.Bold,
                style = MaterialTheme.typography.titleSmall,
            )

            Text(
                text = account.email(),
                modifier = Modifier.fillMaxWidth(),
            )

            Spacer(modifier = Modifier.height(20.dp))

            Text(
                text = "Backend",
                modifier = Modifier.fillMaxWidth(),
                fontWeight = FontWeight.Bold,
                style = MaterialTheme.typography.titleSmall,
            )

            Text(
                text = account.backend(),
                modifier = Modifier.fillMaxWidth()
            )

            Spacer(modifier = Modifier.height(20.dp))

            Text(
                text = stringResource(id = R.string.status_no_colon),
                fontWeight = FontWeight.Bold,
                modifier = Modifier.fillMaxWidth(),
                style = MaterialTheme.typography.titleSmall,
            )

            Text(
                text = accountStatusString(account),
                modifier = Modifier.fillMaxWidth()
            )

            Spacer(modifier = Modifier.height(20.dp))

            HorizontalDivider()

            Spacer(modifier = Modifier.height(20.dp))

            ActionButton(text = stringResource(id = R.string.proxy_settings), onProxyClicked)

            Spacer(modifier = Modifier.height(40.dp))

            if (account.isLoggedOut()) {
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
        }
    }
}