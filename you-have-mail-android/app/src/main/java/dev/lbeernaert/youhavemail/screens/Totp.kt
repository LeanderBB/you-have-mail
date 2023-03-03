package dev.lbeernaert.youhavemail.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
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
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import androidx.navigation.NavController
import dev.lbeernaert.youhavemail.Log
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.ServiceException
import dev.lbeernaert.youhavemail.ServiceView
import dev.lbeernaert.youhavemail.components.ActionButton
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

@Composable
fun Totp(serviceView: ServiceView, navController: NavController) {
    val service = serviceView.getService()!!
    val account = serviceView.getInLoginAccount()!!
    val totp = remember { mutableStateOf(TextFieldValue()) }
    val openDialog = remember { mutableStateOf(false) }
    val coroutineScope = rememberCoroutineScope()


    val onTotpClicked: () -> Unit = {
        openDialog.value = true;

        coroutineScope.launch {
            val exception: ServiceException? = withContext(Dispatchers.IO) {
                var exception: ServiceException? = null
                try {
                    account.submitTotp(totp.value.text)
                } catch (e: ServiceException) {
                    exception = e
                } finally {
                    openDialog.value = false
                }
                exception
            }

            when (exception) {
                null -> {
                    try {
                        service.addAccount(account)
                    } catch (e: ServiceException) {
                        Log.e(e.toString())
                    } finally {
                        serviceView.clearInLoginAccount()
                    }

                    serviceView.requiresAccountRefresh()
                    navController.popBackStack(Routes.Main.route, false)
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
                Text(text = stringResource(id = R.string.totp_title))
            },
                navigationIcon = {
                    IconButton(onClick = {
                        navController.popBackStack()
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
                    .fillMaxSize(),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally,

                ) {
                Text(text = stringResource(R.string.totp_request))

                Spacer(modifier = Modifier.height(20.dp))

                TextField(
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text(text = "TOTP") },
                    value = totp.value,
                    singleLine = true,
                    visualTransformation = PasswordVisualTransformation(),
                    keyboardOptions = KeyboardOptions(
                        keyboardType = KeyboardType.Number,
                        imeAction = ImeAction.Done
                    ),
                    onValueChange = { totp.value = it },
                    keyboardActions = KeyboardActions(onDone = {
                        onTotpClicked()
                    })
                )

                Spacer(modifier = Modifier.height(20.dp))

                ActionButton(text = stringResource(id = R.string.submit), onTotpClicked)
            }
        }
    }
}
