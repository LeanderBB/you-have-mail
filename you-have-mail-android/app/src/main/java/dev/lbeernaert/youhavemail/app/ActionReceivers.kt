package dev.lbeernaert.youhavemail.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log
import dev.lbeernaert.youhavemail.yhmLogError
import dev.lbeernaert.youhavemail.yhmLogInfo


private const val NotificationIDKey = "NotificationID"
private const val TAG = "Receivers"

/**
 * Base class receiver for actions which can be taken on a notification.
 */
open class ActionReceiver(private val name: String, private val description: String) :
    BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        val email = intent.getStringExtra(NotificationIntentEmailKey)
        val action = intent.getStringExtra(NotificationIntentActionKey)
        val backend = intent.getStringExtra(NotificationIntentBackendKey)
        if (email == null || action == null || backend == null) {
            yhmLogError("Received $name broadcast, but email or action was not set")
            return
        }
        val notificationID = intent.getIntExtra(NotificationIDKey, 0)
        if (notificationID != 0) {
            NOTIFICATION_STATE.dismissNotification(context, email, backend, notificationID)
        }
        yhmLogInfo("Received $name broadcast for $email")

        try {
            ActionWorker.queue(context, email, action, description)
        } catch (e: Exception) {
            yhmLogError("Failed to queue action for $email: $e")
            createAndDisplayServiceErrorNotification(
                context,
                "Failed to queue action for $email: $e"
            )
        }
    }
}

/**
 * Broadcast when the user clicks the `Mark Read` notification action.
 */
class MarkReadReceiver : ActionReceiver(name = "MarkRead", "Mark message read") {
    companion object {
        fun newIntent(
            context: Context,
            notificationID: Int,
            email: String,
            action: String
        ): Intent {
            return Intent(context, MarkReadReceiver::class.java).let {
                it.putExtra(NotificationIntentEmailKey, email)
                it.putExtra(NotificationIntentActionKey, action)
                it.putExtra(NotificationIDKey, notificationID)
            }
        }
    }
}

/**
 * Broadcast when the user clicks the `Trash` notification action.
 */
class MoveToTrashReceiver : ActionReceiver(name = "MoveToTrash", "Move message to trash") {
    companion object {
        fun newIntent(
            context: Context,
            notificationID: Int,
            email: String,
            action: String
        ): Intent {
            return Intent(context, MoveToTrashReceiver::class.java).let {
                it.putExtra(NotificationIntentEmailKey, email)
                it.putExtra(NotificationIntentActionKey, action)
                it.putExtra(NotificationIDKey, notificationID)
            }
        }
    }
}

/**
 * Broadcast when the user clicks the `Spam` notification action.
 */
class MoveToSpamReceiver : ActionReceiver("MoveToSpam", "Move message to spam") {
    companion object {
        fun newIntent(
            context: Context,
            notificationID: Int,
            email: String,
            action: String
        ): Intent {
            return Intent(context, MoveToSpamReceiver::class.java).let {
                it.putExtra(NotificationIntentEmailKey, email)
                it.putExtra(NotificationIntentActionKey, action)
                it.putExtra(NotificationIDKey, notificationID)
            }
        }
    }
}

/**
 * Receiver to dismiss group notifications
 */
class DismissGroupNotificationReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val email = intent.getStringExtra(NotificationIntentEmailKey)
        if (email != null) {
            Log.i(TAG, "Dismissing Group Notification for $email")
            // When dismissing a group, all children are also dismissed.
            NOTIFICATION_STATE.dismissGroupNotification(context, email, false)
        }
    }

    companion object {
        fun newIntent(
            context: Context,
            email: String,
        ): Intent {
            return Intent(context, DismissGroupNotificationReceiver::class.java).putExtra(
                NotificationIntentEmailKey,
                email
            )
        }
    }
}

/**
 * Receiver to dismiss a message notifications
 */
class DismissMessageNotificationReceiver : BroadcastReceiver() {
    override fun onReceive(context: Context, intent: Intent) {
        val email = intent.getStringExtra(NotificationIntentEmailKey)
        val backend = intent.getStringExtra(NotificationIntentBackendKey)
        val id = intent.getIntExtra(NotificationIDKey, 0)
        if (email != null && id != 0 && backend != null) {
            Log.i(TAG, "Dismissing Message Notification for $email id=$id")
            NOTIFICATION_STATE.dismissNotification(context, email, backend, id)
        }
    }

    companion object {
        fun newIntent(
            context: Context,
            email: String,
            backend: String,
            notificationID: Int,
        ): Intent {
            return Intent(context, DismissMessageNotificationReceiver::class.java).putExtra(
                NotificationIntentEmailKey,
                email
            ).putExtra(NotificationIDKey, notificationID)
                .putExtra(NotificationIntentBackendKey, backend)
        }
    }
}