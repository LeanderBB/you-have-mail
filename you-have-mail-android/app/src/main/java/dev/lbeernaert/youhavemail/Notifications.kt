package dev.lbeernaert.youhavemail

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
import androidx.core.app.NotificationCompat

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

fun createAccountErrorNotification(
    context: Context,
    email: String,
    err: ServiceException
): Notification {
    val pendingIntent: PendingIntent =
        Intent(context, MainActivity::class.java).let { notificationIntent ->
            PendingIntent.getActivity(context, 0, notificationIntent, PendingIntent.FLAG_IMMUTABLE)
        }

    val builder: NotificationCompat.Builder = NotificationCompat.Builder(
        context,
        NotificationChannelError
    )

    val errorString = serviceExceptionToErrorStr(err, email)

    return builder
        .setContentTitle("You Have Mail")
        .setContentText("$email error: $errorString")
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
    err: ServiceException
): Notification {
    val pendingIntent: PendingIntent =
        Intent(context, MainActivity::class.java).let { notificationIntent ->
            PendingIntent.getActivity(context, 0, notificationIntent, PendingIntent.FLAG_IMMUTABLE)
        }

    val builder: NotificationCompat.Builder = NotificationCompat.Builder(
        context,
        NotificationChannelError
    )

    val errorString = serviceExceptionToErrorStr(err, null)

    return builder
        .setContentTitle("You Have Mail")
        .setContentText("$text: $errorString")
        .setContentIntent(pendingIntent)
        .setAutoCancel(true)
        .setVisibility(Notification.VISIBILITY_PRIVATE)
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
