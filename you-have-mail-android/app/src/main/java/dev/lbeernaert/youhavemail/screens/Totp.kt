package dev.lbeernaert.youhavemail.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.runtime.*
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
import dev.lbeernaert.youhavemail.components.BackgroundTask
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

@Composable
fun Totp(
    onBackClicked: () -> Unit,
    onTotpClicked: suspend (value: String) -> Unit
) {
    val totp = remember { mutableStateOf(TextFieldValue()) }
    val scaffoldState = rememberScaffoldState()
    val openDialog = remember { mutableStateOf(false) }
    val coroutineScope = rememberCoroutineScope()

    val onClick: () -> Unit = {
        coroutineScope.launch {
            openDialog.value = true;
            try {
                onTotpClicked(totp.value.text)
            } catch (err: ServiceException) {
                openDialog.value = false
                Log.e(err.toString())
                coroutineScope.launch {
                    scaffoldState.snackbarHostState.showSnackbar(
                        message = err.message.toString(),
                        duration = SnackbarDuration.Short,
                    )
                }
            } finally {
                openDialog.value = false
            }
        }
    }

    if (openDialog.value) {
        BackgroundTask(
            text = stringResource(id = R.string.submitting_totp)
        )
    } else {
        Scaffold(
            scaffoldState = scaffoldState,
            topBar = {
                TopAppBar(title = {
                    Text(text = stringResource(id = R.string.totp_title))
                },
                    navigationIcon = {
                        IconButton(onClick = onBackClicked) {
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
                        onClick()
                    })
                )

                Spacer(modifier = Modifier.height(20.dp))

                ActionButton(text = stringResource(id = R.string.submit), onClick = onClick)
            }
        }
    }
}
