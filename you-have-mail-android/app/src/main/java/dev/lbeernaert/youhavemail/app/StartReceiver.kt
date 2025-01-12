package dev.lbeernaert.youhavemail.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log
import dev.lbeernaert.youhavemail.Yhm
import dev.lbeernaert.youhavemail.YhmException

class StartReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action == Intent.ACTION_BOOT_COMPLETED) {
            Log.i("BOOT", "Received boot notification")

            try {
                registerWorker(
                    context,
                    YhmInstance.get(context).yhm.pollInterval().toLong() / 60,
                    false
                )
            } catch (e: YhmException) {
                createServiceErrorNotification(
                    context,
                    "Failed to Create Yhm on boot and register work",
                    e
                )
                return
            }
        }
    }
}