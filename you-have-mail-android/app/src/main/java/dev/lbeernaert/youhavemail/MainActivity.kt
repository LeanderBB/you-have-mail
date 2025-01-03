package dev.lbeernaert.youhavemail

import android.Manifest
import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.wrapContentWidth
import androidx.compose.material.AlertDialog
import androidx.compose.material.Button
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Surface
import androidx.compose.material.Text
import androidx.compose.material.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat
import androidx.core.content.ContextCompat.startActivity
import dev.lbeernaert.youhavemail.app.NotificationActionClicked
import dev.lbeernaert.youhavemail.app.NotificationIntentAppNameKey
import dev.lbeernaert.youhavemail.app.NotificationIntentBackendKey
import dev.lbeernaert.youhavemail.app.NotificationIntentEmailKey
import dev.lbeernaert.youhavemail.app.State
import dev.lbeernaert.youhavemail.app.createAndDisplayServiceErrorNotification
import dev.lbeernaert.youhavemail.app.createNotificationChannels
import dev.lbeernaert.youhavemail.app.createServiceErrorNotification
import dev.lbeernaert.youhavemail.app.exportLogs
import dev.lbeernaert.youhavemail.app.getLogPath
import dev.lbeernaert.youhavemail.app.oneshotWorker
import dev.lbeernaert.youhavemail.screens.MainNavController
import dev.lbeernaert.youhavemail.ui.theme.YouHaveMailTheme
import kotlin.system.exitProcess


const val activityLogTag = "activity"

class MainActivity : ComponentActivity() {
    private var mState: State? = null


    override fun onCreate(savedInstanceState: Bundle?) {
        Log.i("BOOT", "OnCreate")
        super.onCreate(savedInstanceState)

        val notificationManager = NotificationManagerCompat.from(this)

        createNotificationChannels(notificationManager)

        val log_init = initLog(getLogPath(this).path)
        if (log_init != null) {
            createServiceErrorNotification(this, "Failed to init log: ${log_init}")
        }


        if (mState == null) {
            try {
                mState = State(this)
            } catch (e: YhmException) {
                yhmLogError("Failed to create state: $e")
                try {
                    createAndDisplayServiceErrorNotification(this, "state init failed", e)
                } catch (e: Exception) {
                    Log.e(activityLogTag, "Failed to create exception")
                }
            }
        }

        if (mState != null) {
            mState!!.migrateAccounts(this)
        }

        var exportLogsLauncher =
            registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
                if (result.resultCode == Activity.RESULT_OK) {
                    result.data?.data?.let {
                        exportLogs(this, it)
                    }
                }
            }

        setContent {
            YouHaveMailTheme {
                // A surface container using the 'background' color from the theme
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colors.background
                ) {
                    val permissionOpenDialog = remember { mutableStateOf(false) }
                    val rationalPermissionOpenDialog = remember { mutableStateOf(false) }

                    if (permissionOpenDialog.value) {
                        ShowSettingDialog(context = this, openDialog = permissionOpenDialog)
                    }

                    var hasNotificationPermission by remember {
                        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                            mutableStateOf(
                                ContextCompat.checkSelfPermission(
                                    this,
                                    Manifest.permission.POST_NOTIFICATIONS
                                ) == PackageManager.PERMISSION_GRANTED
                            )
                        } else mutableStateOf(true)
                    }

                    val launcher = rememberLauncherForActivityResult(
                        contract = ActivityResultContracts.RequestPermission(),
                        onResult = { isGranted ->
                            if (!isGranted) {
                                Log.d(activityLogTag, "Notification permission not granted")
                                if (shouldShowRequestPermissionRationale(Manifest.permission.POST_NOTIFICATIONS)) {
                                    Log.d(activityLogTag, "Show request permission rational")
                                    rationalPermissionOpenDialog.value = true
                                } else {
                                    Log.d(activityLogTag, "Show request permission")
                                    permissionOpenDialog.value = true
                                }
                            } else {
                                Log.d(activityLogTag, "Notification permission granted")
                                hasNotificationPermission = isGranted
                            }
                        }
                    )
                    if (rationalPermissionOpenDialog.value) {
                        ShowRationalPermissionDialog(openDialog = rationalPermissionOpenDialog) {
                            rationalPermissionOpenDialog.value = false
                            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
                                launcher.launch(Manifest.permission.POST_NOTIFICATIONS)
                            }
                        }
                    }

                    if (mState == null) {
                        ShowErrorDialog()
                    } else {
                        MainNavController(
                            context = this,
                            state = mState!!,
                            requestPermissions = {
                                if (!hasNotificationPermission) {
                                    launcher.launch(Manifest.permission.POST_NOTIFICATIONS)
                                }
                            },
                            onPollClicked = {
                                if (!hasNotificationPermission) {
                                    launcher.launch(Manifest.permission.POST_NOTIFICATIONS)
                                }
                                oneshotWorker(this)
                            },
                            onExportLogsClicked = {
                                val intent = Intent(Intent.ACTION_CREATE_DOCUMENT).apply {
                                    addCategory(Intent.CATEGORY_OPENABLE)
                                    type = "application/zip"
                                    putExtra(Intent.EXTRA_TITLE, "you-have-mail-logs.zip")
                                }
                                exportLogsLauncher.launch(intent)
                            }
                        )
                    }
                }
            }
        }
    }

    override fun onDestroy() {
        if (mState != null) {
            mState!!.close()
            mState = null
        }

        super.onDestroy()
    }
}

@Composable
fun ShowRationalPermissionDialog(openDialog: MutableState<Boolean>, onclick: () -> Unit) {
    if (openDialog.value) {
        AlertDialog(
            onDismissRequest = {
                openDialog.value = false
            },
            title = {
                Text(text = stringResource(id = R.string.notification_permission))
            },
            text = {
                Text(stringResource(id = R.string.notification_permission_text2))
            },

            buttons = {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.End,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    TextButton(
                        onClick = {
                            openDialog.value = false
                        }
                    ) {
                        Text(stringResource(id = R.string.notification_permission))
                    }
                    Spacer(modifier = Modifier.width(20.dp))
                    TextButton(
                        onClick = onclick,
                    ) {
                        Text(stringResource(id = R.string.ok))
                    }
                }

            },
        )
    }
}

@Composable
fun ShowSettingDialog(context: Context, openDialog: MutableState<Boolean>) {
    if (openDialog.value) {
        AlertDialog(
            onDismissRequest = {
                openDialog.value = false
            },
            title = {
                Text(text = stringResource(id = R.string.notification_permission))
            },
            text = {
                Text(stringResource(id = R.string.notification_permission_text))
            },

            buttons = {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.End,
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    TextButton(
                        onClick = {
                            openDialog.value = false
                        }
                    ) {
                        Text(stringResource(id = R.string.cancel))
                    }
                    Spacer(modifier = Modifier.width(20.dp))
                    TextButton(
                        onClick = {
                            openDialog.value = false
                            val intent = Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS)
                            intent.data = Uri.parse("package:${context.packageName}")
                            startActivity(context, intent, Bundle())
                        },
                    ) {
                        Text(stringResource(id = R.string.ok))
                    }
                }
            },
        )
    }
}

@Composable
private fun ShowErrorDialog() {
    AlertDialog(
        onDismissRequest = {},
        title = {
            Text(
                modifier = Modifier.fillMaxWidth(),
                textAlign = TextAlign.Center,
                fontSize = 20.sp,
                text = "Error"
            )
        },
        text = {
            Column() {
                Text(
                    stringResource(id = R.string.service_uninitialized),
                    textAlign = TextAlign.Center,
                    modifier = Modifier.fillMaxWidth()
                )
            }
        },
        buttons = {
            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(bottom = 20.dp),
                verticalArrangement = Arrangement.Center,
                horizontalAlignment = Alignment.CenterHorizontally
            ) {
                Button(
                    modifier = Modifier.wrapContentWidth(),
                    onClick = {
                        exitProcess(0)
                    }
                ) {
                    Text("Exit")
                }
            }
        }
    )
}