package dev.lbeernaert.youhavemail.screens

import androidx.compose.foundation.layout.*
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.Text
import androidx.compose.material.TextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.components.ActionButton


@Composable
fun Login(
    backendName: String,
    onBackClicked: () -> Unit,
    onLoginClicked: suspend (email: String, password: String) -> Unit
) {
    var email = remember { mutableStateOf(TextFieldValue()) }
    val password = remember { mutableStateOf(TextFieldValue()) }
    val loginBackgroundLabel = stringResource(id = R.string.login_to_account, email.value.text)

    AsyncScreen(
        title = stringResource(id = R.string.login),
        onBackClicked = onBackClicked
    ) { padding, runTask ->

        val onClick: () -> Unit = {
            runTask(loginBackgroundLabel) {
                onLoginClicked(email.value.text, password.value.text)
            }
        }

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