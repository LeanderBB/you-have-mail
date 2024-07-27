package dev.lbeernaert.youhavemail.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log
import androidx.localbroadcastmanager.content.LocalBroadcastManager

const val START_INTENT= "APP_STARTED"

class StartReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action == Intent.ACTION_BOOT_COMPLETED) {
            Log.i("BOOT", "Received boot notification")

            try {
                val localIntent = Intent(START_INTENT)
                LocalBroadcastManager.getInstance(context).sendBroadcast(localIntent)
            } catch (e: Exception) {
                Log.e("BOOT", "Failed to send intent: $e")
            }
        }
    }
}

