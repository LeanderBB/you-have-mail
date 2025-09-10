package dev.lbeernaert.youhavemail.components

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.Icon
import androidx.compose.material.IconButton
import androidx.compose.material.Text
import androidx.compose.material.TextField
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Visibility
import androidx.compose.material.icons.filled.VisibilityOff
import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.autofill.ContentType
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.contentType
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.text.input.VisualTransformation


@OptIn(ExperimentalComposeUiApi::class)
@Composable
fun PasswordField(
    placeHolder: String,
    state: MutableState<TextFieldValue>,
    onClick: () -> Unit
) {
    val showPassword = remember {
        mutableStateOf(false)
    }
    TextField(
        modifier = Modifier
            .fillMaxWidth()
            .semantics {
                contentType = ContentType.Password
            },
        label = { Text(text = placeHolder) },
        value = state.value,
        singleLine = true,
        visualTransformation = if (!showPassword.value) {
            PasswordVisualTransformation()
        } else {
            VisualTransformation.None
        },
        keyboardOptions = KeyboardOptions(
            keyboardType = KeyboardType.Password,
            imeAction = ImeAction.Done
        ),
        onValueChange = { state.value = it },
        keyboardActions = KeyboardActions(onDone = {
            onClick()
        }),
        trailingIcon = {
            val icon = if (showPassword.value) {
                Icons.Filled.Visibility
            } else {
                Icons.Filled.VisibilityOff
            }

            IconButton(onClick = { showPassword.value = !showPassword.value }) {
                Icon(
                    icon,
                    contentDescription = "Visibility",
                )
            }
        }
    )
}