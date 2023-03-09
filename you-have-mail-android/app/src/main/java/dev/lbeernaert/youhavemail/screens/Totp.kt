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
fun Totp(
    onBackClicked: () -> Unit,
    onTotpClicked: suspend (value: String) -> Unit
) {
    val totp = remember { mutableStateOf(TextFieldValue()) }
    val totpBackgroundLabel = stringResource(id = R.string.submitting_totp)

    AsyncScreen(
        title = stringResource(id = R.string.totp_title),
        onBackClicked = onBackClicked
    ) { padding, runTask ->
        val onClick: () -> Unit = {
            runTask(totpBackgroundLabel) {
                onTotpClicked(totp.value.text)
            }
        }

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
