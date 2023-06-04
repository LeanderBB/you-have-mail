package dev.lbeernaert.youhavemail.service

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Build
import android.util.Log


class StartReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action == Intent.ACTION_BOOT_COMPLETED && getServiceState(context) == ServiceState.STARTED) {
            Intent(context, ObserverService::class.java).also {
                it.action = Actions.START.name
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                    Log.d(
                        serviceLogTag,
                        "Starting the service in >=26 Mode from Boot Completed Broadcast"
                    )
                    context.startForegroundService(it)
                    return
                }
                Log.d(
                    serviceLogTag,
                    "Starting the service in < 26 Mode from Boot Completed Broadcast"
                )
                context.startService(it)
            }
        }
    }
}

