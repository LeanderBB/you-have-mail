package dev.lbeernaert.youhavemail.screens

import android.widget.Toast
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuAnchorType
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import dev.lbeernaert.youhavemail.Auth
import dev.lbeernaert.youhavemail.Protocol
import dev.lbeernaert.youhavemail.Proxy
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.components.ActionButton
import dev.lbeernaert.youhavemail.components.AsyncScreen
import dev.lbeernaert.youhavemail.components.PasswordField

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ProxyScreen(
    onBackClicked: () -> Unit,
    applyButtonText: String,
    onApplyClicked: suspend (Proxy?) -> Unit,
    proxy: Proxy?,
    isLoginRequest: Boolean
) {
    var expanded by remember { mutableStateOf(false) }
    val proxyProtocols = listOf(Protocol.HTTP, Protocol.SOCKS5)
    var selectedIndex by remember {
        mutableStateOf(
            if (proxy != null) {
                if (proxy.protocol == Protocol.HTTP) {
                    0
                } else {
                    1
                }
            } else {
                0
            }
        )
    }

    var useProxy by rememberSaveable { mutableStateOf(proxy != null) }
    var useAuth by rememberSaveable { mutableStateOf(proxy != null && proxy.auth != null) }
    var proxyUrl by rememberSaveable(stateSaver = TextFieldValue.Saver) {
        mutableStateOf(
            TextFieldValue(
                proxy?.host ?: ""
            )
        )
    }
    var proxyPort by rememberSaveable(stateSaver = TextFieldValue.Saver) {
        mutableStateOf(
            TextFieldValue(
                if (proxy != null) {
                    "${proxy.port}"
                } else {
                    ""
                }
            )
        )
    }
    var proxyUser by rememberSaveable(stateSaver = TextFieldValue.Saver) {
        mutableStateOf(
            TextFieldValue(
                if (proxy != null && proxy.auth != null) {
                    proxy.auth!!.user
                } else {
                    ""
                }
            )
        )
    }
    var proxyPassword = remember {
        mutableStateOf(
            TextFieldValue(
                if (proxy != null && proxy.auth != null) {
                    proxy.auth!!.password
                } else {
                    ""
                }
            )
        )
    }
    AsyncScreen(
        title = stringResource(id = R.string.proxy_settings),
        onBackClicked = onBackClicked
    ) { padding, runTask ->
        Column(
            modifier = Modifier
                .padding(padding)
                .padding(20.dp)
                .verticalScroll(rememberScrollState(), true)
                .imePadding()
        ) {

            val context = LocalContext.current

            fun buildProxyObject(
                useProxy: Boolean,
                protocol: Protocol,
                proxyUrl: String,
                proxyPort: String,
                useAuth: Boolean,
                username: String,
                password: String
            ): Proxy? {
                if (!useProxy) {
                    return null
                }

                if (proxyUrl.isEmpty()) {
                    throw RuntimeException("Proxy IP Address can't be empty")
                }

                if (proxyPort.isEmpty()) {
                    throw RuntimeException("Proxy Port can't be empty")
                }

                val auth = if (useAuth) {
                    if (username.isEmpty()) {
                        throw RuntimeException("Proxy Username can't be empty")
                    }

                    if (password.isEmpty()) {
                        throw RuntimeException("Proxy Port can't be empty")
                    }

                    Auth(username, password)
                } else {
                    null
                }

                return Proxy(protocol, auth = auth, host = proxyUrl, port = proxyPort.toUShort())
            }

            val onApplyLabel = stringResource(id = R.string.apply_proxy)
            val onApply: () -> Unit = {
                try {
                    val proxyConfig = buildProxyObject(
                        useProxy,
                        proxyProtocols[selectedIndex],
                        proxyUrl.text,
                        proxyPort.text,
                        useAuth,
                        proxyUser.text,
                        proxyPassword.value.text,
                    )
                    runTask(onApplyLabel) {
                        onApplyClicked(proxyConfig)
                    }
                } catch (e: RuntimeException) {
                    Toast.makeText(context, e.message, Toast.LENGTH_SHORT).show()
                }
            }

            if (isLoginRequest) {
                Text(
                    text = stringResource(id = R.string.proxy_info),
                    style = MaterialTheme.typography.titleMedium
                )
            }

            Row(
                verticalAlignment = Alignment.CenterVertically
            ) {
                Text(
                    text = stringResource(id = R.string.use_proxy),
                    style = MaterialTheme.typography.titleMedium
                )
                Spacer(Modifier.fillMaxWidth(0.9f))
                Switch(checked = useProxy, onCheckedChange = {
                    useProxy = it
                    if (!it) {
                        useAuth = it
                    }
                })
            }

            if (useProxy) {

                ExposedDropdownMenuBox(
                    modifier = Modifier.fillMaxWidth(),
                    expanded = expanded,
                    onExpandedChange = { expanded = !expanded }
                ) {
                    TextField(
                        modifier = Modifier
                            .menuAnchor(ExposedDropdownMenuAnchorType.PrimaryEditable)
                            .fillMaxWidth(),
                        value = proxyProtocols[selectedIndex].toString(),
                        readOnly = true,
                        enabled = useProxy,
                        singleLine = true,
                        onValueChange = {},
                        label = { Text(text = stringResource(id = R.string.proxy_protocol)) },
                        trailingIcon = {
                            ExposedDropdownMenuDefaults.TrailingIcon(
                                expanded = expanded)
                        },
                        colors = ExposedDropdownMenuDefaults.textFieldColors(),
                    )
                    DropdownMenu(
                        modifier = Modifier.exposedDropdownSize(),
                        expanded = expanded,
                        onDismissRequest = { expanded = false }
                    ) {
                        proxyProtocols.forEachIndexed { index, s ->
                            DropdownMenuItem(
                                text = { Text(text = s.toString()) },
                                onClick = {
                                    expanded = false
                                    selectedIndex = index
                                }
                            )
                        }
                    }
                }
                Spacer(modifier = Modifier.height(20.dp))

                TextField(
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text(text = stringResource(id = R.string.ip_address)) },
                    singleLine = true,
                    value = proxyUrl,
                    enabled = useProxy,
                    onValueChange = { proxyUrl = it },
                    keyboardOptions = KeyboardOptions(
                        keyboardType = KeyboardType.Uri,
                        imeAction = ImeAction.Next
                    ),
                )

                Spacer(modifier = Modifier.height(20.dp))

                TextField(
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text(text = stringResource(id = R.string.ip_port)) },
                    singleLine = true,
                    enabled = useProxy,
                    value = proxyPort,
                    onValueChange = { proxyPort = it },
                    keyboardOptions = KeyboardOptions(
                        keyboardType = KeyboardType.Number,
                        imeAction = ImeAction.Next
                    ),
                )

                Spacer(modifier = Modifier.height(20.dp))

                Row(
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text(
                        text = stringResource(id = R.string.proxy_auth),
                        style = MaterialTheme.typography.titleMedium
                    )
                    Spacer(Modifier.fillMaxWidth(0.9f))
                    Switch(checked = useAuth, enabled = useProxy, onCheckedChange = {
                        useAuth = it
                    })
                }

                if (useAuth) {
                    TextField(
                        modifier = Modifier.fillMaxWidth(),
                        label = { Text(text = stringResource(id = R.string.proxy_user)) },
                        singleLine = true,
                        value = proxyUser,
                        enabled = useAuth,
                        onValueChange = { proxyUser = it },
                        keyboardOptions = KeyboardOptions(
                            keyboardType = KeyboardType.Uri,
                            imeAction = ImeAction.Next
                        ),
                    )

                    Spacer(modifier = Modifier.height(20.dp))

                    PasswordField(
                        placeHolder = stringResource(id = R.string.proxy_user_password),
                        state = proxyPassword,
                        onClick = {})
                }
            }

            Spacer(modifier = Modifier.height(20.dp))

            ActionButton(text = applyButtonText, onApply)

        }
    }
}