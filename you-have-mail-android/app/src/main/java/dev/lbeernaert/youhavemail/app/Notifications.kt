package dev.lbeernaert.youhavemail.app

import android.app.Activity
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationChannelGroup
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.graphics.Color
import android.media.AudioAttributes
import android.media.RingtoneManager
import android.text.Html
import android.text.Spanned
import android.util.Log
import androidx.core.app.NotificationCompat
import dev.lbeernaert.youhavemail.MainActivity
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.YhmException
import java.util.concurrent.locks.Lock
import java.util.concurrent.locks.ReentrantLock
import kotlin.concurrent.withLock

const val NotificationActionDismissed = "NotificationDismissed"
const val NotificationActionClicked = "NotificationClicked"
const val NotificationIntentEmailKey = "Email"
const val NotificationIntentBackendKey = "Backend"
const val NotificationIntentAppNameKey = "AppName"

const val NotificationChannelService = "YOU_HAVE_MAIL_SERVICE"
const val NotificationChannelNewMail = "YOU_HAVE_MAIL_NOTIFICATION"
const val NotificationChannelError = "YOU_HAVE_MAIL_ERROR"

const val NotificationGroupNewEmails = "YOU_HAVE_MAIL_NEW_EMAILS"

const val ServiceNotificationID = 1
const val ServiceErrorNotificationID = 2
const val ServiceAccountNotificationsStartID = 3

const val NOTIFICATION_LOG_TAG = "notification"

/**
 * Notification ids for an account.
 *
 * @param newMessages: For new unread messages.
 * @param errors: For error notifications
 */
data class NotificationIds(val newMessages: Int, val statusUpdate: Int, val errors: Int)

/**
 * Unread state for an account.
 *
 * If an account has multiple unread emails, each email will create an entry into the `lines`
 * parameter.
 */
data class UnreadState(var unreadCount: UInt, var lines: ArrayList<Spanned>)

class NotificationState {
    private var idCounter: Int = ServiceAccountNotificationsStartID
    private var accountToIds: HashMap<String, NotificationIds> = HashMap()
    private var unreadState: HashMap<String, UnreadState> = HashMap()
    private var lock: Lock = ReentrantLock()


    /**
     * Get or create notification ids for a given account.
     */
    private fun getOrCreateNotificationIDs(email: String): NotificationIds {
        this.lock.withLock {
            val ids = this.accountToIds[email]
            if (ids != null) {
                return ids
            }

            val newIds = NotificationIds(
                newMessages = idCounter++,
                statusUpdate = idCounter++,
                errors = idCounter++
            )

            this.accountToIds[email] = newIds
            return newIds
        }
    }

    /**
     * Get an account's unread state or update the existing one if any is present.
     */
    private fun getAndUpdateUnreadMessageCount(
        email: String,
        newMessageCount: UInt,
        line: Spanned,
        reset: Boolean
    ): UnreadState {
        this.lock.withLock {
            val result: UnreadState
            if (reset) {
                val state = UnreadState(newMessageCount, arrayListOf(line))
                this.unreadState[email] = state
                result = state
            } else {
                var state = this.unreadState.getOrDefault(email, UnreadState(0u, arrayListOf()))
                state.unreadCount += newMessageCount
                state.lines.add(line)
                this.unreadState[email] = state
                result = state
            }
            return result
        }
    }

    /**
     * Check whether a notification is currently visible.
     */
    private fun isNotificationVisible(context: Context, id: Int): Boolean {
        with(context.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
            for (n in this.activeNotifications) {
                if (n.id == id) {
                    return true
                }
            }
        }

        return false
    }

    /**
     * Create notification for new emails that opens a registered application when interacted with.
     */
    private fun createAlertNotification(
        context: Context,
        email: String,
        backend: String,
        unreadState: UnreadState,
    ): Notification {

        val appName = getAppNameForBackend(backend)

        val clickIntent: PendingIntent? =
            if (appName != null) {
                Intent(context, MainActivity::class.java).let { intent ->
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
                        0,
                        intent,
                        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
                    )
                }
            } else {
                Log.d(
                    NOTIFICATION_LOG_TAG,
                    "No app found for backed '$backend'. No notification action"
                )
                null
            }

        val dismissIntent: PendingIntent =
            Intent().let { intent ->
                intent.action = NotificationActionDismissed
                intent.putExtra(
                    NotificationIntentEmailKey, email
                )
                intent.putExtra(
                    NotificationIntentBackendKey, backend
                )
                PendingIntent.getBroadcast(context, 0, intent, PendingIntent.FLAG_IMMUTABLE)
            }


        val builder: NotificationCompat.Builder = NotificationCompat.Builder(
            context,
            NotificationChannelNewMail
        )

        if (clickIntent != null) {
            builder.setContentIntent(clickIntent)
        }

        if (unreadState.lines.size == 1) {
            return builder
                .setContentTitle("$email has 1 new message")
                .setContentText(unreadState.lines[0])
                .setDeleteIntent(dismissIntent)
                .setAutoCancel(true)
                .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
                .setSmallIcon(R.drawable.ic_stat_alert)
                .setTicker("You Have Mail Alert")
                .build()
        }

        var style = NotificationCompat.InboxStyle()

        for (line in unreadState.lines.reversed()) {
            style = style.addLine(line)
        }

        return builder
            .setContentTitle("$email has ${unreadState.unreadCount} new message(s)")
            .setDeleteIntent(dismissIntent)
            .setStyle(style)
            .setAutoCancel(true)
            .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
            .setSmallIcon(R.drawable.ic_stat_alert)
            .setTicker("You Have Mail Alert")
            .build()
    }

    /**
     * Handle new email arrival and create the appropriate notification.
     */
    fun onNewEmail(
        context: Context,
        account: String,
        backend: String,
        sender: String,
        subject: String
    ) {
        try {
            val ids = getOrCreateNotificationIDs(account)
            val isNotificationActive = isNotificationVisible(context, ids.newMessages)
            val styleText: Spanned =
                Html.fromHtml("<b>$sender:</b> $subject", Html.FROM_HTML_MODE_LEGACY)
            val unreadState =
                getAndUpdateUnreadMessageCount(account, 1u, styleText, !isNotificationActive)
            val notification =
                createAlertNotification(context, account, backend, unreadState)
            with(context.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
                if (this.areNotificationsEnabled()) {
                    notify(ids.newMessages, notification)
                }
            }
        } catch (e: Exception) {
            Log.e(NOTIFICATION_LOG_TAG, "Failed to display notification: $e")
        }
    }

    /**
     * Create a new notification for an account who's session expired.
     */
    fun onLoggedOut(context: Context, email: String) {
        try {
            val notification =
                createAccountStatusNotification(context, "Account $email session expired")
            val ids = getOrCreateNotificationIDs(email)
            with(context.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
                if (this.areNotificationsEnabled()) {
                    notify(ids.statusUpdate, notification)
                }
            }
        } catch (e: Exception) {
            Log.e(NOTIFICATION_LOG_TAG, "Failed to display notification: $e")
        }
    }

    /**
     * Create an error notification for an account.
     */
    fun onError(context: Context, email: String, error: String) {
        try {
            val notification = createAccountErrorNotification(context, email, error)
            val ids = getOrCreateNotificationIDs(email)
            with(context.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
                if (this.areNotificationsEnabled()) {
                    notify(ids.errors, notification)
                }
            }
        } catch (e: Exception) {
            Log.e(NOTIFICATION_LOG_TAG, "Failed to display notification: $e")
        }
    }
}

fun updateServiceNotificationStatus(context: Context, newState: String) {
    val notification = createServiceNotification(context, newState)
    with(context.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
        if (this.areNotificationsEnabled()) {
            notify(ServiceNotificationID, notification)
        }
    }
}

fun createAndDisplayServiceErrorNotification(
    context: Context,
    text: String,
    exception: YhmException
) {
    val notification = createServiceErrorNotification(context, text, exception)
    with(context.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
        if (this.areNotificationsEnabled()) {
            notify(ServiceErrorNotificationID, notification)
        }
    }
}

fun createAndDisplayServiceErrorNotification(
    context: Context,
    text: String,
) {
    val notification = createServiceErrorNotification(context, text)
    with(context.getSystemService(Activity.NOTIFICATION_SERVICE) as NotificationManager) {
        if (this.areNotificationsEnabled()) {
            notify(ServiceErrorNotificationID, notification)
        }
    }
}


fun createServiceNotification(context: Context, state: String): Notification {
    val builder: NotificationCompat.Builder = NotificationCompat.Builder(
        context,
        NotificationChannelService,
    )

    return builder
        .setContentTitle("You Have Mail")
        .setContentText("Background Service $state")
        .setSmallIcon(R.drawable.ic_stat_sync)
        .setVisibility(NotificationCompat.VISIBILITY_SECRET)
        .setCategory(Notification.CATEGORY_SERVICE)
        .setOngoing(true)
        .setTicker("You Have Mail Service")
        .build()
}

private fun createAccountErrorNotification(
    context: Context,
    email: String,
    err: String
): Notification {
    val pendingIntent: PendingIntent =
        Intent(context, MainActivity::class.java).let { notificationIntent ->
            PendingIntent.getActivity(context, 0, notificationIntent, PendingIntent.FLAG_IMMUTABLE)
        }

    val builder: NotificationCompat.Builder = NotificationCompat.Builder(
        context,
        NotificationChannelError
    )

    return builder
        .setContentTitle("You Have Mail")
        .setContentText("$email error: $err")
        .setContentIntent(pendingIntent)
        .setAutoCancel(true)
        .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
        .setSmallIcon(R.drawable.ic_stat_err)
        .setTicker("You Have Mail Alert")
        .build()
}

fun createServiceErrorNotification(
    context: Context,
    text: String,
    err: YhmException
): Notification {
    val pendingIntent: PendingIntent =
        Intent(context, MainActivity::class.java).let { notificationIntent ->
            PendingIntent.getActivity(context, 0, notificationIntent, PendingIntent.FLAG_IMMUTABLE)
        }

    val builder: NotificationCompat.Builder = NotificationCompat.Builder(
        context,
        NotificationChannelError
    )

    val errorString = err.toString()

    return builder
        .setContentTitle("You Have Mail")
        .setContentText("$text: $errorString")
        .setContentIntent(pendingIntent)
        .setAutoCancel(true)
        .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
        .setSmallIcon(R.drawable.ic_stat_err)
        .setTicker("You Have Mail Alert")
        .build()
}

fun createServiceErrorNotification(
    context: Context,
    text: String,
): Notification {
    val pendingIntent: PendingIntent =
        Intent(context, MainActivity::class.java).let { notificationIntent ->
            PendingIntent.getActivity(context, 0, notificationIntent, PendingIntent.FLAG_IMMUTABLE)
        }

    val builder: NotificationCompat.Builder = NotificationCompat.Builder(
        context,
        NotificationChannelError
    )

    return builder
        .setContentTitle("You Have Mail")
        .setContentText("$text")
        .setContentIntent(pendingIntent)
        .setAutoCancel(true)
        .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
        .setSmallIcon(R.drawable.ic_stat_err)
        .setTicker("You Have Mail Alert")
        .build()
}

fun createAccountStatusNotification(context: Context, text: String): Notification {
    val pendingIntent: PendingIntent =
        Intent(context, MainActivity::class.java).let { notificationIntent ->
            PendingIntent.getActivity(
                context,
                0,
                notificationIntent,
                PendingIntent.FLAG_MUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
            )
        }

    val builder: NotificationCompat.Builder = NotificationCompat.Builder(
        context,
        NotificationChannelNewMail,
    )

    return builder
        .setContentTitle("You Have Mail")
        .setContentText(text)
        .setAutoCancel(true)
        .setContentIntent(pendingIntent)
        .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
        .setSmallIcon(R.drawable.ic_stat_alert)
        .setTicker("You Have Mail Alert")
        .build()
}

fun createNotificationChannels(notificationManager: NotificationManager) {
    notificationManager.createNotificationChannelGroup(
        NotificationChannelGroup(
            NotificationGroupNewEmails,
            "New Emails"
        )
    )

    val channelService = NotificationChannel(
        NotificationChannelService,
        "Background Service ",
        NotificationManager.IMPORTANCE_LOW
    ).let {
        it.description = "You Have Mail Background Service"
        it.enableLights(false)
        it.enableVibration(false)
        it.setShowBadge(false)
        it
    }
    notificationManager.createNotificationChannel(channelService)

    val channelAlerter = NotificationChannel(
        NotificationChannelNewMail,
        "New Emails and Status",
        NotificationManager.IMPORTANCE_HIGH
    ).let {
        it.description = "You Have Mail Notifications"
        it.enableLights(true)
        it.lightColor = Color.WHITE
        it.enableVibration(true)
        it.group = NotificationGroupNewEmails
        it.setSound(
            RingtoneManager.getDefaultUri(RingtoneManager.TYPE_NOTIFICATION),
            AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_NOTIFICATION)
                .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
                .build()
        )
        it
    }
    notificationManager.createNotificationChannel(channelAlerter)

    val channelErrors = NotificationChannel(
        NotificationChannelError,
        "Errors",
        NotificationManager.IMPORTANCE_HIGH
    ).let {
        it.description = "You Have Mail Errors"
        it.enableLights(true)
        it.lightColor = Color.RED
        it.enableVibration(true)
        it.setSound(
            RingtoneManager.getDefaultUri(RingtoneManager.TYPE_NOTIFICATION),
            AudioAttributes.Builder().setUsage(AudioAttributes.USAGE_NOTIFICATION)
                .setContentType(AudioAttributes.CONTENT_TYPE_SONIFICATION)
                .build()
        )
        it
    }
    notificationManager.createNotificationChannel(channelErrors)
}
