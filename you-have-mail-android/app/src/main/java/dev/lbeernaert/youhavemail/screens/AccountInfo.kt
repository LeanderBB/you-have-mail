package dev.lbeernaert.youhavemail.screens

import androidx.compose.foundation.layout.*
import androidx.compose.material.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import dev.lbeernaert.youhavemail.*
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.components.ActionButton
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext


@Composable
fun AccountInfo(serviceView: ServiceView, accountIndex: Int, onBackClicked: () -> Unit) {
    val accounts = serviceView.getAccounts()
    if (accounts.size <= accountIndex) {
        return
    }

    val account = accounts[accountIndex]
    val service = serviceView.getService()!!
    val openDialog = rememberSaveable { mutableStateOf(false) }
    val coroutineScope = rememberCoroutineScope()

    val onLogout: () -> Unit = {
        coroutineScope.launch {
            val exception: ServiceException? = withContext(Dispatchers.IO) {
                var exception: ServiceException? = null
                try {
                    service.logoutAccount(account.email())
                } catch (e: ServiceException) {
                    exception = e
                } finally {
                    openDialog.value = false
                }
                exception
            }

            when (exception) {
                null -> {
                    serviceView.requiresAccountRefresh()
                    onBackClicked()
                }
                else -> {
                    Log.e(exception.toString())
                }
            }
        }
    }

    val onDelete: () -> Unit = {
        coroutineScope.launch {
            val exception: ServiceException? = withContext(Dispatchers.IO) {
                var exception: ServiceException? = null
                try {
                    service.logoutAccount(account.email())
                } catch (e: ServiceException) {
                    exception = e
                } finally {
                    openDialog.value = false
                }
                exception
            }

            when (exception) {
                null -> {
                    serviceView.requiresAccountRefresh()
                    onBackClicked()
                }
                else -> {
                    Log.e(exception.toString())
                }
            }
        }
    }

    if (openDialog.value) {
        BackgroundTask(
            text = stringResource(
                R.string.submitting_totp
            )
        )
    } else {
        Scaffold(topBar = {
            TopAppBar(title = {
                Text(text = stringResource(id = R.string.account_title))
            },
                navigationIcon = {
                    IconButton(onClick = {
                        onBackClicked()
                    }) {
                        Icon(
                            imageVector = Icons.Filled.ArrowBack,
                            contentDescription = "Back"
                        )
                    }
                })
        }
        ) { padding ->
            Column(
                modifier = Modifier
                    .padding(padding)
                    .padding(20.dp)
                    .fillMaxSize(),
                verticalArrangement = Arrangement.Top,
                horizontalAlignment = Alignment.CenterHorizontally,

                ) {

                Text(
                    text = account.email(),
                    modifier = Modifier.fillMaxWidth(),
                    style = MaterialTheme.typography.h2,
                )

                Spacer(modifier = Modifier.height(20.dp))

                Text(
                    text = account.backend(),
                    modifier = Modifier.fillMaxWidth()
                )

                Spacer(modifier = Modifier.height(20.dp))

                val statusString = when (account.state()) {
                    ObserverAccountState.OFFLINE -> stringResource(id = R.string.status_offline)
                    ObserverAccountState.LOGGED_OUT -> stringResource(id = R.string.status_logged_out)
                    ObserverAccountState.ONLINE -> stringResource(id = R.string.status_online)
                }
                Text(
                    text = stringResource(id = R.string.status, statusString),
                    modifier = Modifier.fillMaxWidth()
                )

                Spacer(modifier = Modifier.height(40.dp))

                ActionButton(
                    text = stringResource(id = R.string.logout),
                    onLogout,
                    enabled = account.state() != ObserverAccountState.LOGGED_OUT
                )

                Spacer(modifier = Modifier.height(20.dp))

                ActionButton(text = stringResource(id = R.string.delete_account), onDelete)
            }
        }
    }
}