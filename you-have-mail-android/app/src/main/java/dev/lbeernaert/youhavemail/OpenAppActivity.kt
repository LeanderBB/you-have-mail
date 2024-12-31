package dev.lbeernaert.youhavemail

import android.app.Activity
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.os.Bundle
import android.util.Log
import android.widget.Toast
import dev.lbeernaert.youhavemail.app.NOTIFICATION_LOG_TAG
import dev.lbeernaert.youhavemail.app.NOTIFICATION_STATE
import dev.lbeernaert.youhavemail.app.NotificationActionClicked
import dev.lbeernaert.youhavemail.app.NotificationIntentAppNameKey
import dev.lbeernaert.youhavemail.app.NotificationIntentBackendKey
import dev.lbeernaert.youhavemail.app.NotificationIntentEmailKey
import dev.lbeernaert.youhavemail.app.getAppNameForBackend
import dev.lbeernaert.youhavemail.app.newRequestCode

class OpenAppActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val action = intent.action ?: return

        if (action == NotificationActionClicked) {
            val backend = intent.getStringExtra(NotificationIntentBackendKey)!!
            val email = intent.getStringExtra(NotificationIntentEmailKey)!!
            val appName = intent.getStringExtra(NotificationIntentAppNameKey)!!


            // Launch the app for this backend
            Log.d(activityLogTag, "Receive click request for '$email' backend='$backend'")
            try {
                // Dismiss group notifications
                NOTIFICATION_STATE.dismissGroupNotification(this, email)

                Log.d(activityLogTag, "Attempting to launch $appName")
                val appIntent =
                    packageManager.getLaunchIntentForPackage(appName)
                if (appIntent != null) {
                    appIntent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
                    appIntent.addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP)
                    appIntent.addFlags(Intent.FLAG_ACTIVITY_TASK_ON_HOME)
                    appIntent.addFlags(Intent.FLAG_ACTIVITY_RESET_TASK_IF_NEEDED)
                    appIntent.addFlags(Intent.FLAG_ACTIVITY_REORDER_TO_FRONT)
                    startActivity(appIntent)
                } else {
                    Log.e(activityLogTag, "Could not find package $appName")
                    Toast.makeText(this, "Could not find package $appName", Toast.LENGTH_LONG)
                        .show()
                }
            } catch (e: Exception) {
                Log.e(activityLogTag, "Failed to launch $appName for backend $backend: $e")
                yhmLogError("Failed to launch $appName for backend $backend: $e")
                Toast.makeText(
                    this,
                    "Failed to launch $appName for backed $backend",
                    Toast.LENGTH_LONG
                )
                    .show()
            }
            finish()
        }
    }

    companion object {
        fun newIntent(
            context: Context,
            email: String,
            backend: String
        ): PendingIntent? {
            val appName = getAppNameForBackend(backend)
            return if (appName != null) {
                Intent(context, OpenAppActivity::class.java).let { intent ->
                    intent.action = NotificationActionClicked
                    intent.putExtra(
                        NotificationIntentEmailKey, email
                    )
                    intent.putExtra(
                        NotificationIntentBackendKey, backend
                    )
                    intent.putExtra(
                        NotificationIntentAppNameKey,
                        appName
                    )
                    intent.addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP)
                    intent.addFlags(Intent.FLAG_ACTIVITY_CLEAR_TOP)
                    PendingIntent.getActivity(
                        context,
                        newRequestCode(),
                        intent,
                        PendingIntent.FLAG_UPDATE_CURRENT
                    )
                }
            } else {
                Log.d(
                    NOTIFICATION_LOG_TAG,
                    "No app found for backed '$backend'. No notification action"
                )
                null
            }
        }
    }
}