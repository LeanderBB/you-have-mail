package dev.lbeernaert.youhavemail.app

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
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat.getString
import dev.lbeernaert.youhavemail.MainActivity
import dev.lbeernaert.youhavemail.NewEmail
import dev.lbeernaert.youhavemail.OpenAppActivity
import dev.lbeernaert.youhavemail.R
import dev.lbeernaert.youhavemail.Yhm
import dev.lbeernaert.youhavemail.YhmException
import java.util.concurrent.atomic.AtomicInteger
import java.util.concurrent.locks.Lock
import java.util.concurrent.locks.ReentrantLock
import kotlin.concurrent.withLock

const val NotificationActionDismissed = "NotificationDismissed"
const val NotificationActionClicked = "NotificationClicked"
const val NotificationIntentEmailKey = "Email"
const val NotificationIntentBackendKey = "Backend"
const val NotificationIntentAppNameKey = "AppName"
const val NotificationIntentActionKey = "Action"

const val NotificationChannelService = "YOU_HAVE_MAIL_SERVICE"
const val NotificationChannelNewMail = "YOU_HAVE_MAIL_NOTIFICATION"
const val NotificationChannelError = "YOU_HAVE_MAIL_ERROR"

const val NotificationGroupNewEmails = "YOU_HAVE_MAIL_NEW_EMAILS"

const val ServiceNotificationID = 1
const val ServiceErrorNotificationID = 2
const val ServiceAccountNotificationsStartID = 3
const val MailNotificationIDMin = 2000;
const val MailNotificationIDMax = Int.MAX_VALUE - MailNotificationIDMin;

const val NOTIFICATION_LOG_TAG = "notification"

/**
 * Notification ids for an account.
 *
 * @param group: For group notification.
 * @param statusUpdate: For status updates.
 * @param errors: For error notifications
 */
data class NotificationIds(val group: Int, val statusUpdate: Int, val errors: Int)

/**
 * Unread state for an account.
 *
 * If an account has multiple unread emails, each email will create an entry into the `lines`
 * parameter.
 */
data class UnreadState(var notificationIds: HashSet<Int>)

private var RequestCodeCounter: AtomicInteger = AtomicInteger(0)

class NotificationState {
    private var accountToIds: HashMap<String, NotificationIds> = HashMap()
    private var unreadState: HashMap<String, UnreadState> = HashMap()
    private var lock: Lock = ReentrantLock()

    /**
     * Get or create notification ids for a given account.
     */
    private fun getOrCreateNotificationIDs(context: Context, email: String): NotificationIds {
        this.lock.withLock {
            val ids = this.accountToIds[email]
            if (ids != null) {
                return ids
            }

            val accountIds = YhmInstance.get(context).yhm.androidGetOrCreateNotificationIds(email)

            val newIds = NotificationIds(
                group = accountIds.group,
                statusUpdate = accountIds.status,
                errors = accountIds.error,
            )

            this.accountToIds[email] = newIds
            return newIds
        }
    }

    /**
     * Get the next free notification id or create one.
     */
    private fun getNextNotificationID(context: Context, email: String): Int {
        val id = YhmInstance.get(context).yhm.androidNextMailNotificationId(email)

        return MailNotificationIDMin + (id % MailNotificationIDMax)
    }

    /**
     * Mark this notification id as available.
     */
    private fun freeNotificationID(
        context: Context,
        email: String,
        backend: String,
        id: Int
    ) {
        this.lock.withLock {
            val notificationIDs = getOrCreateNotificationIDs(context, email)
            val state = unreadState[email]
            if (state != null) {
                state.notificationIds.remove(id)
                NotificationManagerCompat.from(context).apply {
                    if (state.notificationIds.isEmpty()) {
                        // if no notification ids left cancel group
                        cancel(notificationIDs.group)
                    } else {
                        // update group summary
                        if (this.areNotificationsEnabled()) {
                            val summaryNotification =
                                createGroupNotification(context, email, backend, state)
                            notify(notificationIDs.group, summaryNotification)
                        }
                    }
                }
            }
        }
    }

    /**
     * Dismiss a visible notification.
     */
    fun dismissNotification(context: Context, email: String, backend: String, id: Int) {
        NotificationManagerCompat.from(context).apply {
            cancel(id)
        }
        freeNotificationID(context, email, backend, id)
    }

    /**
     * Dismiss group notification and all its children
     */
    fun dismissGroupNotification(
        context: Context,
        email: String,
        clearChildren: Boolean
    ) {
        lock.withLock {
            val notificationIds = getOrCreateNotificationIDs(context, email)
            NotificationManagerCompat.from(context).apply {
                cancel(notificationIds.group)

                if (clearChildren) {
                    val state = unreadState[email]
                    if (state != null) {
                        for (id in state.notificationIds) {
                            cancel(id)
                        }
                        state.notificationIds.clear()
                    }
                }
            }
        }
    }

    /**
     * Get an account's unread state or update the existing one if any is present.
     */
    private fun getAndUpdateUnreadMessageCount(
        email: String,
        notificationID: Int,
    ): UnreadState {
        this.lock.withLock {
            var state = this.unreadState.getOrDefault(
                email,
                UnreadState(hashSetOf())
            )
            state.notificationIds.add(notificationID)
            this.unreadState[email] = state
            return state
        }
    }

    /**
     * Check whether a notification is currently visible.
     */
    private fun isNotificationVisible(context: Context, id: Int): Boolean {
        NotificationManagerCompat.from(context).apply {
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
    private fun createGroupNotification(
        context: Context,
        email: String,
        backend: String,
        unreadState: UnreadState,
    ): Notification {
        val clickIntent: PendingIntent? = OpenAppActivity.newIntent(context, email, backend)

        val dismissIntent: PendingIntent = PendingIntent.getBroadcast(
            context,
            newRequestCode(),
            DismissGroupNotificationReceiver.newIntent(context, email),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        val builder: NotificationCompat.Builder = NotificationCompat.Builder(
            context,
            NotificationChannelNewMail
        )

        if (clickIntent != null) {
            builder.setContentIntent(clickIntent)
        }

        return builder
            .setContentTitle("$email has ${unreadState.notificationIds.size} new message(s)")
            .setDeleteIntent(dismissIntent)
            .setSubText(email)
            .setGroupAlertBehavior(NotificationCompat.GROUP_ALERT_CHILDREN)
            .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
            .setSmallIcon(R.drawable.ic_stat_alert)
            .setTicker("You Have Mail Alert")
            .setGroup(groupID(email))
            .setGroupSummary(true)
            .build()
    }

    /**
     * Create notification for new emails that opens a registered application when interacted with.
     */
    private fun createNewEmailNotification(
        context: Context,
        email: String,
        backend: String,
        newEmail: NewEmail,
        notificationID: Int,
    ): Notification {

        val clickIntent: PendingIntent? = OpenAppActivity.newIntent(context, email, backend)

        val dismissIntent: PendingIntent =
            PendingIntent.getBroadcast(
                context,
                newRequestCode(),
                DismissMessageNotificationReceiver.newIntent(
                    context,
                    email,
                    backend,
                    notificationID
                ),
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
            )


        val builder: NotificationCompat.Builder = NotificationCompat.Builder(
            context,
            NotificationChannelNewMail
        )

        if (clickIntent != null) {
            builder.setContentIntent(clickIntent)
        }

        if (newEmail.moveToTrashAction != null) {
            val pendingIntent = PendingIntent.getBroadcast(
                context,
                newRequestCode(),
                MoveToTrashReceiver.newIntent(
                    context,
                    notificationID,
                    email,
                    backend,
                    newEmail.moveToTrashAction!!
                ),
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
            )

            builder.addAction(0, getString(context, R.string.action_trash), pendingIntent)
        }

        if (newEmail.moveToSpamAction != null) {
            val pendingIntent = PendingIntent.getBroadcast(
                context,
                newRequestCode(),
                MoveToSpamReceiver.newIntent(
                    context,
                    notificationID,
                    email,
                    backend,
                    newEmail.moveToSpamAction!!
                ),
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
            )

            builder.addAction(0, getString(context, R.string.action_spam), pendingIntent)
        }

        if (newEmail.markAsReadAction != null) {
            val pendingIntent = PendingIntent.getBroadcast(
                context,
                newRequestCode(),
                MarkReadReceiver.newIntent(
                    context,
                    notificationID,
                    email,
                    backend,
                    newEmail.markAsReadAction!!
                ),
                PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
            )

            builder.addAction(0, getString(context, R.string.action_mark_read), pendingIntent)
        }

        return builder
            .setContentTitle(newEmail.sender)
            .setContentText(newEmail.subject)
            .setSubText(email)
            .setDeleteIntent(dismissIntent)
            .setVisibility(NotificationCompat.VISIBILITY_PRIVATE)
            .setSmallIcon(R.drawable.ic_stat_alert)
            .setTicker("You Have Mail Alert")
            .setGroup(groupID(email))
            .setGroupAlertBehavior(NotificationCompat.GROUP_ALERT_CHILDREN)
            .build()
    }

    /**
     * Handle new email arrival and create the appropriate notification.
     */
    fun onNewEmail(
        context: Context,
        account: String,
        backend: String,
        newEmail: NewEmail,
    ) {
        try {
            val accountIDs = getOrCreateNotificationIDs(context, account)
            val messageNotificationID = getNextNotificationID(context, account)
            val unreadState =
                getAndUpdateUnreadMessageCount(
                    account,
                    messageNotificationID,
                )
            val groupNotification = createGroupNotification(context, account, backend, unreadState)
            val newEmailNotification =
                createNewEmailNotification(
                    context,
                    account,
                    backend,
                    newEmail,
                    messageNotificationID,
                )
            NotificationManagerCompat.from(context).apply {
                if (this.areNotificationsEnabled()) {
                    notify(messageNotificationID, newEmailNotification)
                    if (unreadState.notificationIds.size > 1) {
                        notify(accountIDs.group, groupNotification)
                    } else {
                        cancel(accountIDs.group)
                    }
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
            val ids = getOrCreateNotificationIDs(context, email)
            NotificationManagerCompat.from(context).apply {
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
            val ids = getOrCreateNotificationIDs(context, email)
            NotificationManagerCompat.from(context).apply {
                if (this.areNotificationsEnabled()) {
                    notify(ids.errors, notification)
                }
            }
        } catch (e: Exception) {
            Log.e(NOTIFICATION_LOG_TAG, "Failed to display notification: $e")
        }
    }

    /**
     * Create notification group id for a an email
     */
    private fun groupID(email: String): String {
        return "dev.lbeernaert.youhavemail.$email"
    }
}

fun updateServiceNotificationStatus(context: Context, newState: String) {
    val notification = createServiceNotification(context, newState)
    NotificationManagerCompat.from(context).apply {
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
    NotificationManagerCompat.from(context).apply {
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
    NotificationManagerCompat.from(context).apply {
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
            PendingIntent.getActivity(
                context,
                newRequestCode(),
                notificationIntent,
                PendingIntent.FLAG_IMMUTABLE
            )
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
            PendingIntent.getActivity(
                context,
                newRequestCode(),
                notificationIntent,
                PendingIntent.FLAG_IMMUTABLE
            )
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
            PendingIntent.getActivity(
                context,
                newRequestCode(),
                notificationIntent,
                PendingIntent.FLAG_IMMUTABLE
            )
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
                newRequestCode(),
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

fun createNotificationChannels(notificationManager: NotificationManagerCompat) {
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

fun newRequestCode(): Int {
    return RequestCodeCounter.incrementAndGet()
}