package dev.lbeernaert.youhavemail.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.yhmLogError
import dev.lbeernaert.youhavemail.yhmLogInfo


private const val NotificationIDKey = "NotificationID"
private const val TAG = "Receivers"

/**
 * Base class receiver for actions which can be taken on a notification.
 */
open class ActionReceiver(
    private val name: String,
    private val descriptionSuccess: Int,
    private val descriptionFail: Int
) :
    BroadcastReceiver() {

    override fun onReceive(context: Context, intent: Intent) {
        val email = intent.getStringExtra(NotificationIntentEmailKey)
        val action = intent.getStringExtra(NotificationIntentActionKey)
        val backend = intent.getStringExtra(NotificationIntentBackendKey)
        if (email == null || action == null || backend == null) {
            yhmLogError("Received $name broadcast, but email, backend or action was not set")
            Log.e(TAG, "Received $name broadcast, but email, backend or action was not set")
            return
        }
        val notificationID = intent.getIntExtra(NotificationIDKey, 0)
        if (notificationID != 0) {
            NOTIFICATION_STATE.dismissNotification(context, email, backend, notificationID)
        }
        yhmLogInfo("Received $name broadcast for $email")

        try {
            ActionWorker.queue(context, email, action, descriptionSuccess, descriptionFail)
        } catch (e: Exception) {
            yhmLogError("Failed to queue action for $email: $e")
            createAndDisplayServiceErrorNotification(
                context,
                "Failed to queue action for $email: $e"
            )
        }
    }

    companion object {
        fun fillIntentArgs(
            intent: Intent,
            notificationID: Int,
            email: String,
            backend: String,
            action: String,
        ): Intent {
            return intent.putExtra(NotificationIntentEmailKey, email)
                .putExtra(NotificationIntentActionKey, action)
                .putExtra(NotificationIDKey, notificationID)
                .putExtra(NotificationIntentBackendKey, backend)
        }
    }
}

/**
 * Broadcast when the user clicks the `Mark Read` notification action.
 */
class MarkReadReceiver : ActionReceiver(
    name = "MarkRead",
    R.string.msg_mark_read_success,
    R.string.msg_mark_read_fail
) {
    companion object {
        fun newIntent(
            context: Context,
            notificationID: Int,
            email: String,
            backend: String,
            action: String,
        ): Intent {
            return Intent(context, MarkReadReceiver::class.java).let {
                fillIntentArgs(it, notificationID, email, backend, action)
                    // Need to set unique action to prevent caching
                    .setAction("MarkRead-$notificationID-${System.currentTimeMillis()}")
            }
        }
    }
}

/**
 * Broadcast when the user clicks the `Trash` notification action.
 */
class MoveToTrashReceiver : ActionReceiver(
    name = "MoveToTrash",
    R.string.msg_trash_success,
    R.string.msg_trash_fail
) {
    companion object {
        fun newIntent(
            context: Context,
            notificationID: Int,
            email: String,
            backend: String,
            action: String
        ): Intent {
            return Intent(context, MoveToTrashReceiver::class.java).let {
                fillIntentArgs(it, notificationID, email, backend, action)
                    // Need to set unique action to prevent caching
                    .setAction("MoveToTrash-$notificationID-${System.currentTimeMillis()}")
            }
        }
    }
}

/**
 * Broadcast when the user clicks the `Spam` notification action.
 */
class MoveToSpamReceiver : ActionReceiver(
    "MoveToSpam",
    R.string.msg_spam_success,
    R.string.msg_spam_fail
) {
    companion object {
        fun newIntent(
            context: Context,
            notificationID: Int,
            email: String,
            backend: String,
            action: String
        ): Intent {
            return Intent(context, MoveToSpamReceiver::class.java).let {
                fillIntentArgs(it, notificationID, email, backend, action)
                    // Need to set unique action to prevent caching
                    .setAction("MoveToSpam-$notificationID-${System.currentTimeMillis()}")
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
                // Need to set unique action to prevent caching
                .setAction("DismissGroupNotification-${System.currentTimeMillis()}")
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
                // Need to set unique action to prevent caching
                .setAction("DismissMessageNotification-$notificationID-${System.currentTimeMillis()}")
        }
    }
}