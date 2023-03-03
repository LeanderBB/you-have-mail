package dev.lbeernaert.youhavemail

import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.Build
import android.os.Bundle
import android.os.IBinder
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Surface
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import dev.lbeernaert.youhavemail.screens.Main
import dev.lbeernaert.youhavemail.screens.MainNavController
import dev.lbeernaert.youhavemail.service.Actions
import dev.lbeernaert.youhavemail.service.ObserverService
import dev.lbeernaert.youhavemail.service.ServiceState
import dev.lbeernaert.youhavemail.service.getServiceState
import dev.lbeernaert.youhavemail.ui.theme.YouHaveMailTheme

class MainActivity : ComponentActivity(), ServiceConnection {
    private var mBound: Boolean = false
    private var mServiceState = ServiceView()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        actionOnService(Actions.START)
        setContent {
            YouHaveMailTheme {
                // A surface container using the 'background' color from the theme
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colors.background
                ) {
                    MainNavController(service = mServiceState)
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
                Log.i("App Starting the service in >= 26 Mode")
                startForegroundService(it)
                return
            }
            Log.i("App Starting the service in <= 26 Mode")
            startService(it)
        }

    }

    override fun onServiceConnected(name: ComponentName?, service: IBinder?) {
        // We've bound to LocalService, cast the IBinder and get LocalService instance
        Log.i("App Bound to service")
        val binder = service as ObserverService.LocalBinder

        try {
            mServiceState.setService(binder.getService())
            mBound = true
        } catch (e: ServiceException) {
            Toast.makeText(this, "Failed to bind to service: $e", Toast.LENGTH_SHORT).show()
        }
    }

    override fun onServiceDisconnected(name: ComponentName?) {
        Log.i("App disconnected from service")
        mBound = false
        mServiceState.removeService()
    }
}