package dev.lbeernaert.youhavemail

import android.Manifest
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.IBinder
import android.provider.Settings
import android.util.Log
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.*
import androidx.compose.material.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.core.content.ContextCompat
import androidx.core.content.ContextCompat.startActivity
import dev.lbeernaert.youhavemail.screens.MainNavController
import dev.lbeernaert.youhavemail.service.*
import dev.lbeernaert.youhavemail.ui.theme.YouHaveMailTheme


const val activityLogTag = "activity"

class MainActivity : ComponentActivity(), ServiceConnection {
    private var mBound: Boolean = false
    private var mServiceState = ServiceWrapper()


    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        actionOnService(Actions.START)

        onNewIntent(this.intent)

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


                    MainNavController(serviceWrapper = mServiceState, requestPermissions = {
                        if (!hasNotificationPermission) {
                            launcher.launch(Manifest.permission.POST_NOTIFICATIONS)
                        }
                    })
                }
            }
        }
    }

    override fun onStart() {
        super.onStart()

        // Bind to Service
        Intent(this, ObserverService::class.java).also { intent ->
            bindService(intent, this, Context.BIND_AUTO_CREATE)
        }
    }

    override fun onStop() {
        super.onStop()
        unbindService(this)
        mBound = false
    }

    private fun actionOnService(action: Actions) {
        if (getServiceState(this) == ServiceState.STOPPED && action == Actions.STOP) return
        Intent(this, ObserverService::class.java).also {
            it.action = action.name
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                Log.i(activityLogTag, "App Starting the service in >= 26 Mode")
                startForegroundService(it)
                return
            }
            Log.i(activityLogTag, "App Starting the service in <= 26 Mode")
            startService(it)
        }

    }

    override fun onServiceConnected(name: ComponentName?, service: IBinder?) {
        // We've bound to LocalService, cast the IBinder and get LocalService instance
        Log.i(activityLogTag, "App Bound to service")
        val binder = service as ObserverService.LocalBinder

        try {
            mServiceState.setService(binder.getService())
            mBound = true
        } catch (e: ServiceException) {
            val errorText = serviceExceptionToErrorStr(e, null)
            Toast.makeText(this, "Failed to bind to service: $errorText", Toast.LENGTH_SHORT).show()
        }
    }

    override fun onServiceDisconnected(name: ComponentName?) {
        Log.i(activityLogTag, "App disconnected from service")
        mBound = false
        mServiceState.removeService()
    }

    override fun onNewIntent(intent: Intent?) {
        super.onNewIntent(intent)
        if (intent == null) {
            return
        }

        val action = intent.action ?: return

        if (action == notificationActionClicked) {
            val backend = intent.getStringExtra(notificationIntentBackendKey)!!
            val email = intent.getStringExtra(notificationIntentEmailKey)!!
            val appName = intent.getStringExtra(notificationIntentAppNameKey)!!

            // Launch the app for this backend
            Log.d(activityLogTag, "Receive click request for '$email' backend='$backend'")
            try {
                Log.d(serviceLogTag, "Attempting to launch $appName")
                val appIntent =
                    packageManager.getLaunchIntentForPackage(appName)
                if (appIntent != null) {
                    appIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                    appIntent.addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP)
                    appIntent.addFlags(Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED)
                    appIntent.addFlags(Intent.FLAG_ACTIVITY_REORDER_TO_FRONT)
                    startActivity(appIntent)
                } else {
                    Log.e(activityLogTag, "Could not find package $appName")
                }
            } catch (e: Exception) {
                Log.e(activityLogTag, "Failed to launch $appName for backend $backend: $e")
            }
        }

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