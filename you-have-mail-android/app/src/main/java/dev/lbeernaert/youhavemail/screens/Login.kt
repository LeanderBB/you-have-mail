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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import dev.lbeernaert.youhavemail.Log
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.ServiceException
import dev.lbeernaert.youhavemail.components.ActionButton
import dev.lbeernaert.youhavemail.components.BackgroundTask
import kotlinx.coroutines.launch


@Composable
fun Login(
    backendName: String,
    onBackClicked: () -> Unit,
    onLoginClicked: suspend (email: String, password: String) -> Unit
) {

    var email = remember { mutableStateOf(TextFieldValue()) }
    val password = remember { mutableStateOf(TextFieldValue()) }
    val openDialog = remember { mutableStateOf(false) }
    val scaffoldState = rememberScaffoldState()
    val coroutineScope = rememberCoroutineScope()

    val onClick: () -> Unit = {
        coroutineScope.launch {
            openDialog.value = true;
            try {
                onLoginClicked(email.value.text, password.value.text)
            } catch (err: ServiceException) {
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
            text = stringResource(
                R.string.login_to_account,
                email.value.text
            )
        )
    } else {
        Scaffold(
            scaffoldState = scaffoldState,
            topBar = {
                TopAppBar(title = {
                    Text(text = stringResource(id = R.string.add_account_title))
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
                    .padding(20.dp)
                    .fillMaxSize(),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally,

                ) {
                Text(text = stringResource(R.string.login_to_account, backendName))

                Spacer(modifier = Modifier.height(20.dp))

                TextField(
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text(text = "Email") },
                    singleLine = true,
                    value = email.value,
                    onValueChange = { email.value = it },
                    keyboardOptions = KeyboardOptions(
                        keyboardType = KeyboardType.Email,
                        imeAction = ImeAction.Next
                    ),
                )

                Spacer(modifier = Modifier.height(20.dp))

                TextField(
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text(text = "Password") },
                    value = password.value,
                    singleLine = true,
                    visualTransformation = PasswordVisualTransformation(),
                    keyboardOptions = KeyboardOptions(
                        keyboardType = KeyboardType.Password,
                        imeAction = ImeAction.Done
                    ),
                    onValueChange = { password.value = it },
                    keyboardActions = KeyboardActions(onDone = {
                        onClick()
                    })
                )

                Spacer(modifier = Modifier.height(20.dp))

                ActionButton(text = stringResource(id = R.string.login), onClick)
            }
        }
    }
}